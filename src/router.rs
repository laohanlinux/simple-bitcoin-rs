extern crate rocket;
extern crate io_context;
extern crate threadpool;

use blockchain::{BLOCK_PREFIX, BlockChain};
use server;
use transaction::Transaction;
use utxo_set;
use util;
use wallet::Wallet;
use block;
use log::*;

use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use std::sync::atomic::{AtomicBool, Ordering};
use std::collections::HashMap;

pub struct BlockLock {
    bc: Arc<BlockChain>,
    utxos: Arc<utxo_set::UTXOSet>,
}

impl BlockLock {
    fn new(bc: Arc<BlockChain>, utxos: Arc<utxo_set::UTXOSet>) -> BlockLock {
        BlockLock {
            bc: bc,
            utxos: utxos,
        }
    }

    pub fn test_block_hashes(&self) -> Vec<String> {
        let db = &self.bc.db;
        let res = db.get_all_with_prefix(*BLOCK_PREFIX);
        res.into_iter().map(|(k, _)| util::encode_hex(&k)).collect()
    }

    pub fn test_last_block(&self) -> (isize, String, String) {
        (
            self.bc.get_best_height(),
            util::encode_hex(self.bc.get_tip()),
            self.bc.last_block_hash(),
        )
    }

    pub fn tx(&self, txid: &str) -> Option<(String, isize, Transaction)> {
        let best_height = self.best_height();
        let block_iter = self.bc.iter();
        for block in block_iter {
            for ts in &block.transactions {
                if util::encode_hex(&ts.id) == txid {
                    let confirm = best_height - block.height;
                    return Some((util::encode_hex(block.hash), confirm, ts.clone()));
                }
            }
        }
        None
    }

    pub fn block_hashes(&self) -> Vec<String> {
        let hashes = &self.bc.get_block_hashes();
        hashes
            .into_iter()
            .map(|item| util::encode_hex(item))
            .collect()
    }

    pub fn create_new_utxo_transaction(
        &self,
        from_wallet: &Wallet,
        to: &str,
        amount: isize,
        spend_utxos: Option<HashMap<String, Vec<isize>>>,
    ) -> Result<Transaction, String> {
        let utxos = &self.utxos;
        let tx = Transaction::new_utxo_transaction(
            &from_wallet,
            to.to_owned(),
            amount,
            utxos,
            spend_utxos,
        );
        tx.map_err(|e| format!("{:?}", e))
    }

    pub fn add_new_block(
        &self,
        new_block: &block::Block,
        from_central_node: bool,
    ) -> Result<bool, String> {
        let block_hash = &new_block.hash;
        if self.bc.get_block(&block_hash).is_some() {
            //debug!(LOG, "{} has exist, ignore", util::encode_hex(&block_hash));
            return Ok(true);
        }

        if from_central_node {
            warn!(
                LOG,
                "may be delete fork block, hash: {}, height: {}",
                util::encode_hex(block_hash),
                new_block.height
            );
            // clear confect blocks
            self.bc.delete_blocks(&new_block.hash, new_block.height);
        }

        // TODO check new block
        let verify = new_block.transactions.iter().any(|ts| {
            // transactions' input should in utxo
            if ts.is_coinbase() {
                return true;
            }

            ts.vin.iter().any(|vin| {
                self.utxos
                    .utxo(&vin.txid)
                    .map(|outputs| outputs.outputs.get(&vin.vout).is_some())
                    .is_some()
            })
        });
        if !verify {
            return Err("block is invalid".to_string());
        }
        self.bc.add_block(new_block).map(|_| false)
    }

    pub fn block(&self, hash: &str) -> Option<block::Block> {
        self.bc.get_block(&util::decode_hex(hash))
    }
    pub fn block_with_height(&self, height: isize) -> Option<block::Block> {
        let block_iter = self.bc.iter();
        for block in block_iter {
            if block.height == height {
                return Some(block);
            }
        }
        None
    }

    pub fn download_blocks(&self) -> Vec<block::Block> {
        self.bc.all_blocks()
    }

    pub fn best_height(&self) -> isize {
        self.bc.get_best_height()
    }

    pub fn balance(&self, addr: &str) -> HashMap<String, String> {
        let mut balance = 0;
        let pub_key_hash = util::decode_base58(addr.to_owned());
        let pub_key_hash = &pub_key_hash[1..(pub_key_hash.len() - 4)];
        let utxos = self.utxos.find_utxo(pub_key_hash);
        for out in utxos {
            balance += out.value;
        }
        let mut res: HashMap<String, String> = HashMap::new();
        res.entry("addr".to_owned()).or_insert(addr.to_string());
        res.entry("balance".to_owned()).or_insert(
            balance.to_string(),
        );
        res
    }

    pub fn unspend_utxo(&self) -> Vec<String> {
        let db = self.bc.db.clone();
        let utxos = db.get_all_with_prefix("utxo-");
        utxos
            .into_iter()
            .map(|(k, _)| util::encode_hex(&k))
            .collect()
    }

    // TODO Opz mining step
    pub fn mine_new_block(
        &self,
        mine_addr: String,
        mem_pool: &mut HashMap<String, Transaction>,
    ) -> Result<String, String> {
        let mut txs = vec![];
        let cbtx = Transaction::new_coinbase_tx(mine_addr, "".to_owned());
        txs.push(cbtx);
        for ts in mem_pool.values() {
            if self.bc.verify_transaction(ts) {
                txs.push(ts.clone());
            }
        }
        if txs.len() <= 1 {
            return Err("no transactions".to_string());
        }

        let new_block = self.bc.mine_block(&txs);
        if new_block.is_err() {
            // delete dirty transaction
            for ts in &txs {
                mem_pool.remove(&util::encode_hex(&ts.id));
            }
            return Err(format!("{:?}", new_block.err()));
        }
        let new_block = new_block.unwrap();
        self.utxos.update(&new_block);
        for ts in &txs {
            mem_pool.remove(&util::encode_hex(&ts.id));
        }
        Ok(util::encode_hex(&new_block.hash))
    }

    // TODO Opz mining step
    pub fn mine_new_block2(
        &self,
        mine_addr: String,
        mem_pool: &HashMap<String, Transaction>,
    ) -> Result<Receiver<block::Block>, String> {
        let mut txs = vec![];
        let cbtx = Transaction::new_coinbase_tx(mine_addr, "".to_owned());
        txs.push(cbtx);
        for ts in mem_pool.values() {
            if self.bc.verify_transaction(ts) {
                txs.push(ts.clone());
            }
        }
        if txs.len() <= 1 {
            return Err("no transactions".to_string());
        }

        // start mine thread backend
        let bc = Arc::clone(&self.bc);
        let (send, recv) = channel();

        thread::spawn(move || {
            let new_block = bc.mine_block(&txs);
            if new_block.is_err() {
                error!(
                    LOG,
                    "mine block thread happend error: {:?}",
                    new_block.err()
                );
                drop(send);
            } else {
                let new_block = new_block.unwrap();
                send.send(new_block.clone()).unwrap();
                info!(
                    LOG,
                    "{} block will be send by mine thread channel",
                    util::encode_hex(&new_block.hash)
                );
            }
        });

        Ok(recv)
    }

    pub fn update_utxo(&self, new_block: &block::Block) {
        self.utxos.update(new_block);
    }

    pub fn block_chain(&self) -> Arc<BlockChain> {
        Arc::clone(&self.bc)
    }
}

pub struct BlockState {
    pub bc: Arc<Mutex<BlockLock>>,
    pub known_nodes: Arc<Mutex<Vec<String>>>,
    pub mining_address: Arc<String>,
    pub block_in_transit: Arc<Mutex<Vec<Vec<u8>>>>,
    pub mem_pool: Arc<Mutex<HashMap<String, Transaction>>>,
    pub run_mining: Arc<AtomicBool>,
    pub local_node: Arc<String>,
}

impl BlockState {
    pub fn new(
        bc: BlockChain,
        local_node: String,
        central_node: String,
        mining_address: String,
    ) -> BlockState {

        let bc = Arc::new(bc);
        let utxos = utxo_set::UTXOSet::new(Arc::clone(&bc));
        utxos.reindex();

        let mut known_nodes = vec![central_node.clone()];
        if known_nodes[0] != local_node.clone() {
            known_nodes.push(local_node.clone());
        }

        let bc_lock = BlockLock::new(Arc::clone(&bc), Arc::new(utxos));

        BlockState {
            bc: Arc::new(Mutex::new(bc_lock)),
            known_nodes: Arc::new(Mutex::new(known_nodes)),
            mining_address: Arc::new(mining_address),
            block_in_transit: Arc::new(Mutex::new(vec![])),
            mem_pool: Arc::new(Mutex::new(HashMap::new())),
            run_mining: Arc::new(AtomicBool::new(false)),
            local_node: Arc::new(local_node),
        }
    }
}

pub fn init_router(addr: &str, port: u16, block_chain: BlockState) {
    let mut conf = rocket::Config::new(rocket::config::Environment::Production)
        .expect("invalid config");
    conf.set_address(addr).unwrap();
    conf.set_port(port);
    rocket::Rocket::custom(conf, true)
        .manage(block_chain)
        .mount("/", routes![server::index])
        .mount("/", routes![server::handle_node_list])
        .mount("/", routes![server::handle_mempool_list])
        .mount("/", routes![server::handle_list_block])
        .mount("/", routes![server::handle_addr])
        .mount("/", routes![server::handle_get_blocks])
        .mount("/", routes![server::handle_inv])
        .mount("/", routes![server::handle_tx])
        .mount("/", routes![server::handle_version])
        .mount("/", routes![server::handle_block])
        .mount("/", routes![server::handle_get_block_data])
        .mount("/", routes![server::handle_generate_secrectkey])
        .mount("/", routes![server::handle_valid_pubkey])
        .mount("/", routes![server::handle_transfer])
        .mount("/", routes![server::handle_balance])
        .mount("/", routes![server::handle_unspend_utxos])
        .mount("/", routes![server::handle_info_block])
        .mount("/", routes![server::handle_tx_info])
        .mount("/", routes![server::handle_test_list_block])
        .mount("/", routes![server::handle_test_last_block])
        .mount("/", routes![server::handle_test_mempool_blocks])
        .mount("/", routes![server::handle_test_download_blocks])
        .launch();
}
