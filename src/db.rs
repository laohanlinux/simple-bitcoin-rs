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
        let mut read_opt = DBReadOptions::new().unwrap();
        read_opt.set_fill_cache(false);
        let enc_key = enc_key(key, prefix);
        let db_clone = self.db.clone();
        let db = db_clone.lock().unwrap();
        match db.get_opts(&enc_key, read_opt) {
            Ok(v) => v,
            Err(e) => {
                let str = format!("{:?}", e);
                panic!(str)
            }
        }
    }

    pub fn put_with_prefix(&self, key: &[u8], value: &[u8], prefix: &str) {
        let mut write_opt = DBWriteOptions::new().unwrap();
        write_opt.set_sync(true);
        let enc_key = enc_key(key, prefix);
        let db_clone = self.db.clone();
        let mut db = db_clone.lock().unwrap();
        println!("db 增加新的数据=> {}", util::encode_hex(&enc_key));
        db.put_opts(&enc_key, value, write_opt).unwrap();
    }

    // return value not included prefix
    pub fn get_all_with_prefix(&self, prefix: &str) -> Vec<(Vec<u8>, Vec<u8>)> {
        let db_clone = self.db.clone();
        let mut db = db_clone.lock().unwrap();
        let kvs: Vec<(Vec<u8>, Vec<u8>)> = db.iter().unwrap().alloc().collect();
        kvs.into_iter()
            .filter(|ref tuple| {
                let enc_key = &tuple.0;
                let prefix = Vec::from(prefix);
                if enc_key.len() < prefix.len() {
                    return false;
                }
                enc_key.starts_with(&prefix)
            })
            .map(|ref tuple| {
                let (_, origin_key) = dec_key(&tuple.0, prefix);
                (origin_key.to_vec(), tuple.1.clone())
            })
            .collect()
    }

    pub fn delete(&self, key: &[u8], prefix: &str) {
        let mut write_opt = DBWriteOptions::new().unwrap();
        write_opt.set_sync(true);
        let enc_key = enc_key(key, prefix);
        let db_clone = self.db.clone();
        let mut db = db_clone.lock().unwrap();
        println!("db 删除数据=> {}", util::encode_hex(&enc_key));
        db.delete_opts(&enc_key, write_opt).unwrap();
    }
}

pub fn enc_key(key: &[u8], prefix: &str) -> Vec<u8> {
    let mut enc_key = Vec::from(prefix);
    enc_key.extend_from_slice(key);
    enc_key
}

pub fn dec_key<'a>(enc_key: &'a [u8], prefix: &str) -> (&'a [u8], &'a [u8]) {
    let prefix_bit = Vec::from(prefix).len();
    assert!(enc_key.len() >= prefix_bit);
    (&enc_key[..prefix_bit], &enc_key[prefix_bit..])
}
#[cfg(test)]
mod tests {
    extern crate tempdir;
    use self::tempdir::TempDir;
    use std::io::{self, Write};
    use blockchain::leveldb_rs::DBOptions;
    #[test]
    fn enc_dec_key() {
        let key = vec![0, 3, 4, 6, 123];
        let prefix = "Shift";
        let enc_key = super::enc_key(&key, prefix);
        let (p, k) = super::dec_key(&enc_key, prefix);
        println!("{:?}", String::from_utf8(p.to_vec()));
    }

    #[test]
    fn db() {
        let path = "/tmp/block_chain/blockchain.db/";
        let db = super::DBStore::new(path, DBOptions::new());
    }

    /*    #[test]
    fn db() {
        let path = super::TempDir::new("/tmp/").unwrap();
        let db = super::DBStore::new(&path.path());
        db.put_with_prefix(b"hello", b"word", "L");
        let value = db.get_with_prefix(b"hello", "L").unwrap();
        writeln!(
            io::stdout(),
            "value => {:?}",
            String::from_utf8(value).unwrap()
        );
    }
    */
}
