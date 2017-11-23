extern crate secp256k1;

use self::secp256k1::key::SecretKey;

use super::block::*;
use super::transaction::*;
use super::db::DBStore;
use super::util;
use super::utxo_set;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

lazy_static! {
    static ref LAST_BLOCK_HASH_KEY:&'static [u8]  = b"last_block".as_ref();
    static ref LAST_BLOCK_HASH_PREFIX:&'static str = "l-";
    static ref BLOCK_PREFIX:&'static str  = "blocks";
    static ref GENESIS_COINBASE_DATA:&'static str = "The Times 03/Jan/2009 Chancellor on brink of second bailout for banks";
}

pub const DBFILE: &str = "{}/blockchain.db";

// TODO add locker locks blockchain update
pub struct BlockChain {
    tip: Arc<Mutex<Vec<u8>>>,
    pub db: Arc<DBStore>,
}

impl BlockChain {
    // build a new block chain from genesis block
    pub fn create_blockchain(address: String, node: String) -> BlockChain {
        let cbtx = Transaction::new_coinbase_tx(address, (*GENESIS_COINBASE_DATA).to_string());
        let genesis_block = Block::new_genesis_block(cbtx);

        let db_file = rt_format!(DBFILE, &node).unwrap();
        let mut prefixs = Vec::<String>::new();
        {
            prefixs.push((*LAST_BLOCK_HASH_PREFIX).to_string());
            prefixs.push((*BLOCK_PREFIX).to_string());
            prefixs.push(utxo_set::UTXO_BLOCK_PREFIX.to_string());
        }
        let db = DBStore::new(&db_file, prefixs);

        // store genesis_block into db
        let value = Block::serialize(&genesis_block);
        let key = genesis_block.hash;
        db.put_with_prefix(&key, &value, *BLOCK_PREFIX);

        // store last block hash into db
        db.put_with_prefix(*LAST_BLOCK_HASH_KEY, &key, *LAST_BLOCK_HASH_PREFIX);

        BlockChain {
            tip: Arc::new(Mutex::new(key)),
            db: Arc::new(db),
        }
    }

    pub fn new_blockchain(node: String) -> BlockChain {
        let db_file = rt_format!(DBFILE, node).unwrap();
        let mut prefixs = Vec::<String>::new();
        {
            prefixs.push((*LAST_BLOCK_HASH_PREFIX).to_string());
            prefixs.push((*BLOCK_PREFIX).to_string());
            prefixs.push(utxo_set::UTXO_BLOCK_PREFIX.to_string());
        }

        let db = DBStore::new(&db_file, prefixs);
        let tip = db.get_with_prefix(*LAST_BLOCK_HASH_KEY, *LAST_BLOCK_HASH_PREFIX)
            .unwrap();
        BlockChain {
            tip: Arc::new(Mutex::new(tip)),
            db: Arc::new(db),
        }
    }

    pub fn last_block_hash(&self) -> String {
        let last_hash = self.db
            .clone()
            .get_with_prefix(*LAST_BLOCK_HASH_KEY, *LAST_BLOCK_HASH_PREFIX)
            .unwrap();
        util::encode_hex(&last_hash)
    }

    // TODO check block all transaction valid
    pub fn add_block(&self, block: &Block) -> Result<(), String> {
        if self.db
            .clone()
            .get_with_prefix(&block.hash, *BLOCK_PREFIX)
            .is_some()
        {
            return Ok(());
        }

        let last_hash = &self.db
            .get_with_prefix(*LAST_BLOCK_HASH_KEY, *LAST_BLOCK_HASH_PREFIX)
            .unwrap();
        let last_block_data = &self.db
            .get_with_prefix(&last_hash, *BLOCK_PREFIX)
            .unwrap();

        let last_block = Block::deserialize_block(&last_block_data);

        if block.height < last_block.height {
            return Err("block's height too small".to_string());
        }
        if block.height == last_block.height {
            return Err("generate hard fork".to_string());
        }
        if block.height != last_block.height + 1 {
            return Err("block's height too big".to_string());
        }
        // check height 
        let block_data = Block::serialize(&block);
        self.db.put_with_prefix(
            &block.hash,
            &block_data,
            *BLOCK_PREFIX,
        );

        &self.db.put_with_prefix(
            *LAST_BLOCK_HASH_KEY,
            &block.hash,
            *LAST_BLOCK_HASH_PREFIX,
            );
        let tip = self.tip.clone();
        {
            let mut tip = tip.lock().unwrap();
            *tip = block.hash.clone();
        }
    
        Ok(())
    }

    // TODO optizme it
    pub fn find_transaction(&self, id: &[u8]) -> Option<Transaction> {
        let block_iter = self.iter();
        for block in block_iter {
            for transaction in &block.transactions {
                if util::compare_slice_u8(&transaction.id, id) {
                    return Some(transaction.clone());
                }
            }
        }
        None
    }

    // FindUTXO finds all unspent transaction outputs and returns transactions with spent outputs removed
    pub fn find_utxo(&self) -> Option<HashMap<String, TXOutputs>> {
        // utxo 未花费
        let mut utxo: HashMap<String, TXOutputs> = HashMap::new();
        // 已花费，对应于输入，也就是说如果“输出”能找到一个“输入”引用它，那么它就是被消费了，
        // 从最新的区块开始往前找，每找到一个区块，则将这些区块的输入放到spend_txos中，
        // 如果某个“输出”在spend_txos中找到一个引用它的“输入”，则表示该输入被消费了
        let mut spent_txos: HashMap<String, Vec<isize>> = HashMap::new();
        let block_iter = self.iter();
        for block in block_iter {
            for transaction in &block.transactions {
                let txid = &util::encode_hex(&transaction.id);
                let mut out_idx = 0;
                for vout in &transaction.vout {
                    let txos = spent_txos.get(txid);
                    let mut find = false;

                    if txos.is_some() {
                        for vout_idx in txos.unwrap() {
                            if out_idx == *vout_idx {
                                find = true;
                                break;
                            }
                        }
                    }
                    // Was the output spent
                    if !find {
                        utxo.entry(txid.clone())
                            .or_insert(TXOutputs { outputs: Box::new(HashMap::new()) })
                            .outputs
                            .insert(out_idx, vout.clone());
                    }
                    out_idx += 1;
                }

                // TODO may be has error issue
                if !transaction.is_coinbase() {
                    for input in &transaction.vin {
                        let in_txid = util::encode_hex(&input.txid);
                        spent_txos.entry(in_txid).or_insert(vec![]).push(input.vout);
                    }
                }
            }
        }
        Some(utxo)
    }

    // TODO why not use self.tip
    pub fn get_best_height(&self) -> isize {
        let last_hash = self.db
            .clone()
            .get_with_prefix(*LAST_BLOCK_HASH_KEY, *LAST_BLOCK_HASH_PREFIX)
            .unwrap();
        let last_block_data = self.db
            .clone()
            .get_with_prefix(&last_hash, *BLOCK_PREFIX)
            .unwrap();
        let last_block = Block::deserialize_block(&last_block_data);
        last_block.height
    }

    pub fn get_block(&self, block_hash: &[u8]) -> Option<Block> {
        let block_data = self.db.clone().get_with_prefix(block_hash, *BLOCK_PREFIX);
        block_data.map(|v| Block::deserialize_block(&v))
    }

    pub fn get_block_hashes(&self) -> Vec<Vec<u8>> {
        let block_iter = self.iter();
        let mut blocks = vec![];
        for block in block_iter {
            blocks.push(block.hash);
        }
        blocks
    }

    pub fn mine_block(&self, transactions: &Vec<Transaction>) -> Result<Block, String> {
        for tx in transactions {
            println!("need to check transaction id is {:?}", &tx.id);
            if !self.verify_transaction(&tx) {
                panic!("ERROR: Invalid transaction")
            }
        }

        let last_hash = self.db
            .clone()
            .get_with_prefix(*LAST_BLOCK_HASH_KEY, *LAST_BLOCK_HASH_PREFIX)
            .unwrap();
        let last_block_data = self.db
            .clone()
            .get_with_prefix(&last_hash, *BLOCK_PREFIX)
            .unwrap();
        let last_block = Block::deserialize_block(&last_block_data);
        let last_height = last_block.height;
        let new_block = Block::new(transactions.clone(), last_hash, last_height + 1);
        let new_block_data = Block::serialize(&new_block);
        self.add_block(&new_block).map(|_| new_block)

        /*self.db.clone().put_with_prefix(
            &new_block.hash,
            &new_block_data,
            *BLOCK_PREFIX,
        );
        self.db.clone().put_with_prefix(
            *LAST_BLOCK_HASH_KEY,
            &new_block.hash,
            *LAST_BLOCK_HASH_PREFIX,
        );
        let tip = self.tip.clone();
        {
            let mut tip = tip.lock().unwrap();
            *tip = new_block.hash.clone();
        }*/
    }

    pub fn iter(&self) -> IterBlockchain {
        let tip = self.tip.clone();
        let tip = tip.lock().unwrap();
        let current_hash = tip.clone();
        let current_block_data = self.db
            .clone()
            .get_with_prefix(&current_hash, *BLOCK_PREFIX)
            .unwrap();
        let current_block = Block::deserialize_block(&current_block_data);
        let db = self.db.clone();
        IterBlockchain::new(db, current_block)
    }

    pub fn sign_transaction(&self, tx: &mut Transaction, secret_key: &SecretKey) -> Result<(), String> {
        let mut prev_txs: HashMap<String, Transaction> = HashMap::new();
        for vin in &tx.vin {
            if let Some(prev_tx) = self.find_transaction(&vin.txid) {
                prev_txs.insert(util::encode_hex(&prev_tx.id), prev_tx);
            }else {
                return Err(format!("not found the transation, txid:{}", util::encode_hex(&vin.txid)));
            }
        }
        tx.sign(&secret_key, &prev_txs);
        Ok(())
    }

    // TODO why coinbase need not verify
    pub fn verify_transaction(&self, tx: &Transaction) -> bool {
        if tx.is_coinbase() {
            return true;
        }

        let mut prevs_tx: HashMap<String, Transaction> = HashMap::new();
        for vin in &tx.vin {
            let prev_tx = {
                let res_pre_tx = self.find_transaction(&vin.txid);
                if res_pre_tx.is_none(){
                    return false;         
                }
                res_pre_tx.unwrap()
            };
            prevs_tx.insert(util::encode_hex(&prev_tx.id), prev_tx);
        }
        tx.verify(&prevs_tx)
    }
}

pub struct IterBlockchain {
    next: Option<Block>,
    db: Arc<DBStore>,
}

impl IterBlockchain {
    pub fn new(db: Arc<DBStore>, next: Block) -> IterBlockchain {
        IterBlockchain {
            db: db,
            next: Some(next),
        }
    }
}

impl Iterator for IterBlockchain {
    type Item = Block;
    fn next(&mut self) -> Option<Self::Item> {
        if self.next.is_none() {
            return None;
        }

        let current_block = self.next.take().unwrap();
        let prev_block_data = self.db.get_with_prefix(
            &current_block.prev_block_hash,
            *BLOCK_PREFIX,
        );
        if prev_block_data.is_some() {
            let prev_block_data = prev_block_data.unwrap();
            let prev_block = Block::deserialize_block(&prev_block_data);
            self.next = Some(prev_block);
        }
        Some(current_block)
    }
}
