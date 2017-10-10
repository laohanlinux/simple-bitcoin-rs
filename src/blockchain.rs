extern crate leveldb_rs;

use self::leveldb_rs::*;

use super::block::*;
use super::transaction::*;
use super::utxo_set::UTXOSet;
use super::db::DBStore;
use super::util;

use std::io::Write;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

lazy_static! {
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
        self.db.borrow().put_with_prefix(
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

    // TODO optizme it
    pub fn find_transaction(&self, id: &[u8]) -> Option<Transaction> {
        let db = self.db.borrow();
        let result = db.get_all_with_prefix(*BLOCK_PREFIX);
        if result.len() == 0 {
            return None;
        }
        for kv in &result {
            if util::compare_slice_u8(&kv.0, id) {
                return Some(Transaction::deserialize_transaction(&kv.1));
            }
        }
        None
    }

    // FindUTXO finds all unspent transaction outputs and returns transactions with spent outputs removed
    pub fn find_utxo(&self) -> Option<HashMap<String, TXOutput>> {
//        let mut utxo = HashMap::new();
        let mut spent_txos: HashMap<String, Vec<_>> = HashMap::new();
        let result = self.db.borrow().get_all_with_prefix(*BLOCK_PREFIX);
        let block_iter = self.iter();
        for block in block_iter {
            for transaction in &block.transactions {
                let txid = util::encode_hex(&transaction.id);
//
//                for vout in &transaction.vout {
//                    let txos = spent_txos.get(&txid);
//                    let vout_value = vout.value;
//                    txos.map(|v|{
//                        for vout_idex in v {
//                            if vout_idex == vout_value {
//                                return v;
//                            }
//                        }
//                        v
//                    });
//                }

                if !transaction.is_coinbase() {
                    for input in &transaction.vin {
                        let in_txid = util::encode_hex(&input.txid);
                        let new_value = {
                            let value = spent_txos.get_mut(&in_txid);
                            value.map_or(vec![input.vout], |v| { v.push(input.vout); vec![]})
                        };
                        spent_txos.insert(in_txid, new_value);
                    }
                }
            }
        }
        None
    }

    pub fn iter(&self) -> IterBlockchain {
        let current_hash = &self.tip;
        let current_block_data = self.db.borrow().get_with_prefix(current_hash, *BLOCK_PREFIX).unwrap();
        let current_block = Block::deserialize_block(&current_block_data);
        let db = self.db.borrow().clone();
        IterBlockchain::new(db, current_block)
    }
}

struct IterBlockchain {
    next: Option<Block>,
    db: DBStore,
}

impl IterBlockchain {
    pub fn new(db: DBStore, next: Block) -> IterBlockchain {
        IterBlockchain { db: db, next: Some(next) }
    }
}

impl Iterator for IterBlockchain {
    type Item = Block;
    fn next(&mut self) -> Option<Self::Item> {
        let current_block = self.next.take().unwrap();
        let prev_block_data = self.db.get_with_prefix(&current_block.prev_block_hash, *BLOCK_PREFIX);
        if prev_block_data.is_some() {
            let prev_block_data = prev_block_data.unwrap();
            let prev_block = Block::deserialize_block(&prev_block_data);
            self.next = Some(prev_block);
        }
        self.next.clone()
    }
}
