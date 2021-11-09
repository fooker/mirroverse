use std::sync::atomic::{AtomicU64, Ordering};
use std::future::Future;
use anyhow::Result;
use std::collections::BTreeSet;
use tokio::sync::Mutex;

pub struct Coorinator {
    next: AtomicU64,

    transactions: Mutex<BTreeSet<u64>>,
}

impl Coorinator {
    pub fn with_index(index: u64) -> Self {
        return Self {
            next: AtomicU64::new(index),
            transactions: Mutex::new(BTreeSet::new()),
        };
    }
    
    pub async fn process<F, G, R>(&self, f: F) -> Result<(R, Option<u64>)>
    where
        G: Future<Output=Result<R>>,
        F: FnOnce(u64) -> G,
    {
        let index = self.next.fetch_add(1, Ordering::SeqCst);

        self.transactions
            .lock().await
            .insert(index);

        let result = f(index).await?;

        let mut transactions = self.transactions
            .lock().await;

        // If the index is the lowest transaction -> commit it
        let result = if transactions.first() == Some(&index) {
            (result, Some(index))
        } else {
            (result, None)
        };

        transactions.remove(&index);

        return Ok(result);
    }
}
