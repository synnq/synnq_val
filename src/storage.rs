use rocksdb::{ DB, Options };
use std::sync::Arc;

#[derive(Clone)]
pub struct Storage {
    db: Arc<DB>,
}

impl Storage {
    pub fn new(path: &str) -> Self {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        Storage { db: Arc::new(DB::open(&opts, path).unwrap()) }
    }

    pub fn store_data(&self, key: &str, value: &str) {
        self.db.put(key, value).unwrap();
    }
}
