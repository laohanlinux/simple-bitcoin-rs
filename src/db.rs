extern crate leveldb_rs;
extern crate tempdir;
extern crate lmdb_rs as lmdb;

use self::leveldb_rs::*;
use self::lmdb::*;

use super::util;

use std::sync::{Arc, Mutex};
use std::path::Path;

#[derive(Clone)]
pub struct DBStore {
    pub db: Arc<Mutex<Environment>>,
}

impl DBStore {
    pub fn new(path: &str, max_db: usize) -> DBStore {
        let env = EnvBuilder::new().max_dbs(max_db).open(path, 0o777).unwrap();
        DBStore { db: Arc::new(Mutex::new(env)) }
    }

    pub fn get_with_prefix(&self, key: &[u8], prefix: &str) -> Option<Vec<u8>> {
        let db_clone = self.db.clone();
        let db = db_clone.lock().unwrap();
        let db_handle = db.create_db(prefix, DbFlags::empty()).unwrap();
        let reader = db.get_reader().unwrap();
        let db = reader.bind(&db_handle);
        Some(db.get(&key).unwrap())
    }

    pub fn put_with_prefix(&self, key: &[u8], value: &[u8], prefix: &str) {
        let db_clone = self.db.clone();
        let db = db_clone.lock().unwrap();
        let db_handle = db.create_db(prefix, DbFlags::empty()).unwrap();

        let txn = db.new_transaction().unwrap();
        let db = txn.bind(&db_handle);
        db.set(&key, &value).unwrap();
    }

    // return value not included prefix
    pub fn get_all_with_prefix(&self, prefix: &str) -> Vec<(Vec<u8>, Vec<u8>)> {
        let db_clone = self.db.clone();
        let db = db_clone.lock().unwrap();
        let db_handle = db.create_db(prefix, DbFlags::empty()).unwrap();
        let reader = db.get_reader().unwrap();
        let db = reader.bind(&db_handle);
        let mut cursor = db.new_cursor().unwrap();

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
