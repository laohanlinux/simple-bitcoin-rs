extern crate leveldb_rs;
extern crate tempdir;

use self::leveldb_rs::*;
use self::tempdir::TempDir;

use std::sync::{Arc, Mutex};
use std::path::Path;

/*
 *In blocks, the key -> value pairs are:

'b' + 32-byte block hash -> block index record
'f' + 4-byte file number -> file information record
'l' -> 4-byte file number: the last block file number used
'R' -> 1-byte boolean: whether we're in the process of reindexing
'F' + 1-byte flag name length + flag name string -> 1 byte boolean: various flags that can be on or off
't' + 32-byte transaction hash -> transaction index record
In chainstate, the key -> value pairs are:

'c' + 32-byte transaction hash -> unspent transaction output record for that transaction
'B' -> 32-byte block hash: the block hash up to which the database represents the unspent transaction outputs
 *
 *
 *
 * 32-byte block-hash -> Block structure (serialized)
'l' -> the hash of the last block in a chain

 * **/

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
        let dec_key = dec_key(key, prefix);
        let db_clone = self.db.clone();
        let db = db_clone.lock().unwrap();
        match db.get(&dec_key) {
            Ok(v) => v,
            Err(e) => {
                let str = format!("{:?}", e);
                panic!(str)
            }
        }
    }

    pub fn put_with_prefix(&self, key: &[u8], value: &[u8], prefix: &str) {
        let dec_key = dec_key(key, prefix);
        let db_clone = self.db.clone();
        let mut db = db_clone.lock().unwrap();
        db.put(&dec_key, value).unwrap();
    }

    pub fn get_all_with_prefix(&self, prefix: &str) -> Vec<(Vec<u8>, Vec<u8>)> {
        let db_clone = self.db.clone();
        let mut db = db_clone.lock().unwrap();
        let kvs: Vec<(Vec<u8>, Vec<u8>)> = db.iter().unwrap().alloc().collect();
        kvs.into_iter()
            .filter(|ref tuple| {
                let k = &String::from_utf8(tuple.0.to_vec()).unwrap();
                k.starts_with(prefix)
            })
            .collect()
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
        writeln!(
            io::stdout(),
            "value => {:?}",
            String::from_utf8(value).unwrap()
        );
    }
}
