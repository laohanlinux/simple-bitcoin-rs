extern crate tempdir;
extern crate rocksdb;

use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use util::encode_hex;
use log::*;

#[derive(Clone)]
pub struct DBStore {
    pub db: Arc<HashMap<String, Mutex<rocksdb::DB>>>,
}

impl DBStore {
    pub fn new(path: &str, prefixs: Vec<String>) -> DBStore {
        let mut db_map = HashMap::new();
        for prefix in prefixs {
            let db_path = format!("{}/{}", path, prefix);
            let db = rocksdb::DB::open_default(db_path).unwrap();
            debug!(LOG, "db init {}", &prefix);
            db_map.insert(prefix, Mutex::new(db));
        }
        DBStore { db: Arc::new(db_map) }
    }

    pub fn get_with_prefix(&self, key: &[u8], prefix: &str) -> Option<Vec<u8>> {
        //info!(LOG, "get prefix {}, key {}", prefix, encode_hex(key));
        let db = self.db.get(prefix).unwrap();
        let db = db.lock().unwrap();
        match db.get(key) {
            Ok(Some(value)) => {
                let v = value.to_vec();
                Some(v)
            }
            Ok(None) => None,
            Err(e) => panic!(e),
        }
    }

    pub fn put_with_prefix(&self, key: &[u8], value: &[u8], prefix: &str) {
        //info!(LOG, "put prefix {}, key {}", prefix, encode_hex(key));
//        if prefix.starts_with("blocks") {
//            info!(LOG, "prefix {}", encode_hex(&key));
//        }
        let db = self.db.get(prefix).unwrap();
        let db = db.lock().unwrap();
        db.put(key, value).unwrap();
    }

    // return value not included prefix
    pub fn get_all_with_prefix(&self, prefix: &str) -> Vec<(Vec<u8>, Vec<u8>)> {
        //info!(LOG, "getall prefix {}", prefix);
        let db = self.db.get(prefix).unwrap();
        let db = db.lock().unwrap();
        let mut iter = db.raw_iterator();
        iter.seek_to_first();
        let mut kvs = Vec::new();
        while iter.valid() {
            kvs.push((iter.key().unwrap(), iter.value().unwrap()));
            iter.next();
        }
        kvs
    }

    pub fn delete(&self, key: &[u8], prefix: &str) {
        //info!(LOG, "delete prefix {}, key {}", prefix, encode_hex(key));
        let db = self.db.get(prefix).unwrap();
        let db = db.lock().unwrap();
        db.delete(key).unwrap()
    }
}
