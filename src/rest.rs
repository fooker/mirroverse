pub use things::Thing;
pub use images::Image;
pub use files::File;

pub mod things {
    use std::collections::HashMap;

    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct Tag {
        pub name: String,
        pub tag: String,
    }

    #[derive(Debug, Deserialize)]
    pub struct Creator {
        pub id: u64,
        pub name: String,
        pub first_name: String,
        pub last_name: String,
    }

    #[derive(Debug, Deserialize)]
    pub struct Detail {
        pub name: String,
        pub r#type: String,
        pub data: Option<Vec<HashMap<String, String>>>,
    }

    #[derive(Debug, Deserialize)]
    pub struct Thing {
        pub id: u64,
        pub name: String,

        pub description: String,
        pub instructions: String,
        pub details: String,

        pub details_parts: Vec<Detail>,

        pub tags: Vec<Tag>,

        pub creator: Creator,
        pub license: String,

        pub files_url: String,
        pub images_url: String,
    }
}

pub mod images {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct Size {
        pub r#type: String,
        pub size: String,
        pub url: String,
    }

    #[derive(Debug, Deserialize)]
    pub struct Image {
        pub id: u64,
        pub name: String,
        pub sizes: Vec<Size>,
    }
}

pub mod files {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    pub struct Size {
        pub r#type: String,
        pub size: String,
        pub url: String,
    }

    #[derive(Debug, Deserialize)]
    pub struct File {
        pub id: u64,

        pub name: String,
        pub size: u64,

        pub public_url: String,
        pub direct_url: Option<String>,
    }
}
