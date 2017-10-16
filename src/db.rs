extern crate leveldb_rs;
extern crate tempdir;

use self::leveldb_rs::*;

use super::util;

use std::sync::{Arc, Mutex};
use std::path::Path;

#[derive(Clone)]
pub struct DBStore {
    pub db: Arc<Mutex<DB>>,
}

impl DBStore {
    pub fn new(path: &str, db_opt: DBOptions) -> Self {
        let db = DB::open_with_opts(Path::new(path), db_opt).unwrap();
        DBStore { db: Arc::new(Mutex::new(db)) }
    }

    pub fn get_with_prefix(&self, key: &[u8], prefix: &str) -> Option<Vec<u8>> {
        let enc_key = enc_key(key, prefix);
        let db_clone = self.db.clone();
        let db = db_clone.lock().unwrap();
        match db.get(&enc_key) {
            Ok(v) => v,
            Err(e) => {
                let str = format!("{:?}", e);
                panic!(str)
            }
        }
    }

    pub fn put_with_prefix(&self, key: &[u8], value: &[u8], prefix: &str) {
        let enc_key = enc_key(key, prefix);
        let db_clone = self.db.clone();
        let mut db = db_clone.lock().unwrap();
        println!("put key {:?}, prefix:{}", &enc_key, prefix);
        db.put(&enc_key, value).unwrap();
    }

    pub fn get_all_with_prefix(&self, prefix: &str) -> Vec<(Vec<u8>, Vec<u8>)> {
        println!("find all: {}", prefix);
        let db_clone = self.db.clone();
        let mut db = db_clone.lock().unwrap();
        let kvs: Vec<(Vec<u8>, Vec<u8>)> = db.iter().unwrap().alloc().collect();
        kvs.into_iter()
            .filter(|ref tuple| {
                let enc_key = &tuple.0;
                let prefix = Vec::from(prefix);
                println!("prefix {:?}, {:?}", prefix, enc_key);
                if enc_key.len() < prefix.len() {
                    return false;
                }
                enc_key.starts_with(&prefix)
            })
            .collect()
    }

    pub fn delete(&self, key: &[u8]) {
        let db_clone = self.db.clone();
        let mut db = db_clone.lock().unwrap();
        db.delete(key).unwrap();
    }
}

pub fn enc_key(key: &[u8], prefix: &str) -> Vec<u8> {
    let mut enc_key = Vec::from(prefix);
    enc_key.extend_from_slice(key);
    enc_key
}

pub fn dec_key<'a>(enc_key: &'a [u8], prefix: &str) -> (&'a [u8], &'a [u8]) {
    let prefix_bit = Vec::from(prefix).len();
    println!("{}, {}, {}", enc_key.len(), prefix_bit, prefix);
    assert!(enc_key.len() >= prefix_bit);
    (&enc_key[..prefix_bit], &enc_key[prefix_bit..])
}
//#[cfg(test)]
//mod tests {
//    extern crate tempdir;
//    use self::tempdir::TempDir;
//    use std::io::{self, Write};
//    #[test]
//    fn db() {
//        let path = super::TempDir::new("/tmp/").unwrap();
//        let db = super::DBStore::new(&path.path());
//        db.put_with_prefix(b"hello", b"word", "L");
//        let value = db.get_with_prefix(b"hello", "L").unwrap();
//        writeln!(
//            io::stdout(),
//            "value => {:?}",
//            String::from_utf8(value).unwrap()
//        );
//    }
//}
