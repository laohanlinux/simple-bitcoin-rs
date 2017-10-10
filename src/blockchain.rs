extern crate leveldb_rs;

use self::leveldb_rs::*;

use super::block::*;
use super::transaction::*;
use super::db::DBStore;

use std::io::Write;
use std::cell::RefCell;

lazy_static!{
//    static ref MAX_NONCE: U256 = 10.into();
    static ref LAST_BLOCK_HASH_KEY:&'static [u8]  = b"".as_ref();
    static ref LAST_BLOCK_HASH_PREFIX:&'static str = "l-";
    static ref BLOCK_PREFIX:&'static str  = "blocks";
    static ref GENESIS_COINBASE_DATA:&'static str = "The Times 03/Jan/2009 Chancellor on brink of second bailout for banks";
}

const DBFILE: &str = "blockchain_{:?}.db";

//#[derive]
pub struct BlockChain {
    tip: Vec<u8>,
    db: RefCell<DBStore>,
}

impl BlockChain {
    pub fn create_blockchain(address: String, node: String) -> BlockChain {
        let cbtx = Transaction::new_coinbase_tx(address, (*GENESIS_COINBASE_DATA).to_string());
        let mut genesis_block = Block::new_genesis_block(cbtx);

        let mut db_opt = DBOptions::new().expect("error create options");
        db_opt.set_error_if_exists(true).set_create_if_missing(true);

        let db_file = rt_format!(DBFILE, node).unwrap();
        let mut db = DBStore::new(&db_file, db_opt);

        // store genesis_block into db
        let value = Block::serialize(&genesis_block);
        let key = genesis_block.hash;
        db.put_with_prefix(&key, &value, *BLOCK_PREFIX);

        // store last block hash into db
        db.put_with_prefix(*LAST_BLOCK_HASH_KEY, &key, *LAST_BLOCK_HASH_PREFIX);

        BlockChain {
            tip: key,
            db: RefCell::new(db),
        }
    }

    pub fn new_blockchain(node: String) -> BlockChain {
        let mut db_opt = DBOptions::new().expect("error create options");
        db_opt.set_create_if_missing(false);
        let db_file = rt_format!(DBFILE, node).unwrap();
        let mut db = DBStore::new(&db_file, db_opt);
        let tip = db.get_with_prefix(*LAST_BLOCK_HASH_KEY, *LAST_BLOCK_HASH_PREFIX)
            .unwrap();
        BlockChain {
            tip: tip,
            db: RefCell::new(db),
        }
    }


    pub fn add_block(&mut self, block: Block) {
        if self.db.borrow().get_with_prefix(&block.hash, *BLOCK_PREFIX).is_some() {
            return;
        }

        let block_data = Block::serialize(&block);
        self.db.get_mut().put_with_prefix(
            &block.hash,
            &block_data,
            *BLOCK_PREFIX,
        );

        let last_hash = self.db.borrow().get_with_prefix(*LAST_BLOCK_HASH_KEY, *LAST_BLOCK_HASH_PREFIX).unwrap();
        let last_block_data = self.db.borrow().get_with_prefix(&last_hash, *BLOCK_PREFIX).unwrap();

        let last_block = Block::deserialize_block(&last_block_data);

        if block.height > last_block.height {
            self.db.borrow().put_with_prefix(
                *LAST_BLOCK_HASH_KEY,
                &block.hash,
                *LAST_BLOCK_HASH_PREFIX,
            );
            self.tip = block.hash.clone();
        }
    }
}
