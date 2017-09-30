extern crate leveldb_rs;
extern crate tempdir;

use self::leveldb_rs::*;
use self::tempdir::TempDir;

use std::sync::{Arc, Mutex};
use std::path::Path;

const BLOCK_PREFIX: &str = "b-";
const UTXO_PREFIX: &str = "l-";

pub struct DBStore {
    db: Arc<Mutex<DB>>,
}

impl DBStore {
    pub fn new(path: &Path) -> Self {
        let db = DB::create(path).unwrap();
        DBStore { db: Arc::new(Mutex::new(db)) }
    }

    pub fn get_with_prefix(&self, key: &[u8], prefix: &str) -> Option<Vec<u8>> {
        let mut dec_key = dec_key(key, prefix);
        let db_clone = self.db.clone();
        let db = db_clone.lock().unwrap();
        match db.get(&dec_key) {
            Ok(v) => v,
            Err(e) => None,
        }
    }

    pub fn put_with_prefix(&self, key: &[u8], value: &[u8], prefix: &str) {
        let mut dec_key = dec_key(key, prefix);
        let db_clone = self.db.clone();
        let mut db = db_clone.lock().unwrap();
        db.put(&dec_key, value).unwrap();
    }

    pub fn get_all_with_prefix(&self, prefix: &str) -> Vec<(Vec<u8>, Vec<u8>)>{
        let db_clone = self.db.clone();
        let mut db = db_clone.lock().unwrap();
        let kvs: Vec<(Vec<u8>, Vec<u8>)> = db.iter().unwrap().alloc().collect();
        kvs.into_iter().filter(|ref tuple| {
            let k = &String::from_utf8(tuple.0.to_vec()).unwrap();
            k.starts_with(prefix)
        }).collect()
    }
}

pub fn dec_key(key: &[u8], prefix: &str) -> Vec<u8> {
    let mut dec_key = Vec::from(prefix);
    dec_key.extend_from_slice(key);
    dec_key
}


#[cfg(test)]
mod tests {
    extern crate tempdir;
    use self::tempdir::TempDir;
    use std::io::{self, Write};
    #[test]
    fn db() {
        let path = super::TempDir::new("/tmp/").unwrap();
        let db = super::DBStore::new(&path.path());
        db.put_with_prefix(b"hello", b"word", "L");
        let value = db.get_with_prefix(b"hello", "L").unwrap();
        writeln!(io::stdout(), "value => {:?}",String::from_utf8(value).unwrap());
    }
}