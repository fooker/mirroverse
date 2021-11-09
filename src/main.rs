#![feature(map_first_last)]

use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use futures::Future;
use futures::stream::StreamExt;
use reqwest::{Client, StatusCode};
use slug::slugify;
use tokio::io::AsyncWriteExt;
use tracing::{instrument, Instrument};
use tracing_subscriber::filter::LevelFilter;

use crate::coordinator::Coorinator;

mod rest;
mod model;
mod coordinator;

#[derive(Parser)]
pub struct Opts {
    #[clap(short, long)]
    pub start: Option<u64>,

    #[clap(short, long, parse(from_occurrences))]
    pub verbose: u32,

    #[clap(short, long, default_value = "4")]
    pub workers: u32,

    #[clap(short, long)]
    pub token: String,

    #[clap(short, long, default_value = "content")]
    pub path: PathBuf,
}

fn filename(p: impl AsRef<str>) -> String {
    return if let Some((name, ext)) = p.as_ref().rsplit_once('.') {
        format!("{}.{}", slugify(name), ext)
    } else {
        slugify(p)
    };
}

#[instrument(name = "download", skip(client))]
async fn download(client: &Client, url: &str, path: impl AsRef<Path> + Debug) -> Result<()> {
    let response = client.get(url)
        .send().await?;

    if response.status() == StatusCode::NOT_FOUND ||
        response.status() == StatusCode::FORBIDDEN {
        return Ok(());
    }

    let mut out = tokio::fs::File::create(path).await?;

    let mut response = response
        .error_for_status()?
        .bytes_stream();
    while let Some(chunk) = response.next().await {
        out.write_all(&chunk?).await?;
    }

    return Ok(());
}

async fn mirror(client: &Client, token: &str, path: impl AsRef<Path>, id: u64) -> Result<bool> {
    let path = path.as_ref().join((id / 1000 * 1000).to_string()).join(id.to_string());

    let thing = client.get(format!("https://api.thingiverse.com/things/{}", id))
        .bearer_auth(&token)
        .send().await?;

    if thing.status() == StatusCode::NOT_FOUND ||
        thing.status() == StatusCode::FORBIDDEN {
        return Ok(false);
    }

    if tokio::fs::metadata(&path).await.is_ok() {
        return Ok(false);
    } else {
        tokio::fs::create_dir_all(&path).await?;
    }

    let thing = thing
        .error_for_status()?
        .json::<rest::Thing>().await?;

    let images = client.get(&thing.images_url)
        .bearer_auth(&token)
        .send().await?
        .error_for_status()?
        .json::<Vec<rest::Image>>().await?;

    let files = client.get(&thing.files_url)
        .bearer_auth(&token)
        .send().await?
        .error_for_status()?
        .json::<Vec<rest::File>>().await?;

    let thing = model::Thing {
        id: thing.id,
        name: thing.name,
        description: thing.description,
        instructions: thing.instructions,
        details: thing.details,
        tags: thing.tags.into_iter()
            .map(|tag| tag.name)
            .collect(),
        creator: model::Creator {
            id: thing.creator.id,
            name: thing.creator.name,
            first_name: thing.creator.first_name,
            last_name: thing.creator.last_name,
        },
        license: thing.license,
    };
    let json = serde_json::to_vec_pretty(&thing)?;
    tokio::fs::write(path.join("info.json"), json).await?;

    let images = async {
        let images_path = path.join("images");
        tokio::fs::create_dir(&images_path).await?;
        for image in images {
            if let Some(size) = image.sizes.into_iter()
                .find(|size| size.r#type == "display" && size.size == "large") {
                download(&client,
                         &size.url,
                         images_path.join(filename(image.name)))
                    .await?;
            }
        }

        Result::<()>::Ok(())
    }.instrument(tracing::debug_span!("download images"));

    let files = async {
        let files_path = path.join("files");
        tokio::fs::create_dir(&files_path).await?;
        for file in files {
            download(&client,
                     &file.direct_url.unwrap_or(file.public_url),
                     files_path.join(filename(file.name)))
                .await?;
        }

        Result::<()>::Ok(())
    }.instrument(tracing::debug_span!("download files"));

    images.await?;
    files.await?;

    return Ok(true);
}

async fn retry<F, G, R>(retries: usize, f: F) -> Result<R>
    where
        F: Fn(usize) -> G,
        G: Future<Output=Result<R>>,
{
    let mut retry = 0;
    loop {
        retry += 1;
        match f(retry).await {
            Ok(result) => {
                return Ok(result);
            }
            Err(err) => {
                if retry < retries {
                    continue;
                } else {
                    return Err(err);
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    tracing_subscriber::fmt()
        .with_max_level(match opts.verbose {
            0 => LevelFilter::WARN,
            1 => LevelFilter::INFO,
            2 => LevelFilter::DEBUG,
            _ => LevelFilter::TRACE,
        })
        .init();

    tokio::fs::create_dir_all(&opts.path).await?;

    let client = reqwest::Client::builder().build()?;

    let start = if let Some(start) = opts.start { start } else {
        let path = opts.path.join("index");

        if tokio::fs::metadata(&path).await.is_ok() {
            let start = tokio::fs::read_to_string(path).await?;
            start.parse()?
        } else { 1 }
    };

    let coordinator = Arc::new(Coorinator::with_index(start));

    let workers = (0..opts.workers)
        .map(|worker| {
            let coordinator = coordinator.clone();

            let client = client.clone();

            let token = opts.token.clone();
            let path = opts.path.clone();

            return Box::pin(async move {
                loop {
                    let result = coordinator.process(|id| {
                        let client = &client;
                        let token = &token;
                        let path = &path;

                        retry(3, move |retry| {
                            mirror(client, token, path, id)
                                .instrument(tracing::info_span!("mirror", worker, id, retry))
                        })
                    }).await?;

                    if let (true, Some(index)) = result {
                        tracing::info!("Successful mirrored index: {}", index);

                        // Commit successful index
                        tokio::fs::write(path.join("index"), index.to_string())
                            .await?;
                    }
                }
            });
        });

    return futures::future::select_all(workers).await.0;
}
