extern crate slog;
extern crate slog_term;
extern crate prettytable;
extern crate typemap;
extern crate chan;
extern crate serde_json;

use self::prettytable::Table;
use self::prettytable::row::Row;
use self::prettytable::cell::Cell;

use super::util;
use super::log::*;
use super::wallets::Wallets;
use super::wallet::Wallet;
use super::blockchain::BlockChain;
use super::utxo_set::UTXOSet;
use super::proof_of_work::ProofOfWork;
use super::transaction;
use super::router;
use super::server;
use super::pool;
use super::command;

use std::fs;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::thread;
use std::ops::Fn;

pub fn create_wallet(node: String, del_old: bool) {
    if del_old {
        fs::remove_file(&node).unwrap();
    }
    let wallets = Wallets::new().unwrap();
    wallets.save_to_file(&node);
    info!(
        LOG,
        "All your wallet  address:",
    );
    let address = wallets.list_address();
    address.into_iter().fold(0, |acc, addr| {
        info!(LOG, "地址[{:?}]=> {:?}", acc, addr);
        acc + 1
    });
}

pub fn add_wallet(node: String) {
    let mut wallets = Wallets::new_wallets(node.clone()).unwrap();
    let new_address = wallets.create_wallet();
    fs::remove_file(&node).unwrap();
    wallets.save_to_file(&node);
    info!(LOG, "new wallet's address is {}", new_address);
}

pub fn open_wallet(node: String) {
    let wallets = Wallets::new_wallets(node).unwrap();
    info!(
        LOG,
        "All your wallet  address:",
    );
    let address = wallets.list_address();
    address.into_iter().fold(0, |acc, addr| {
        let addr_vec = util::decode_base58(addr.clone());
        info!(LOG, "地址[{:?}]=> {:?}, {:?}", acc, addr, addr_vec);
        acc + 1
    });
}

pub fn create_blockchain(address: String, node: String) -> Result<(), String> {
    if !Wallet::validate_address(address.clone()) {
        return Err("address is invalid".to_owned());
    }
    let blockchain = BlockChain::create_blockchain(address, node);
    info!(LOG, "block chain disk data create successfully.");
    let ref_bc = Arc::new(blockchain);
    UTXOSet::new(ref_bc.clone()).reindex();
    info!(LOG, "utxoset reindexs successfully.");
    Ok(())
}

pub fn address_check(address: String) -> Result<(), String> {
    if !Wallet::validate_address(address) {
        return Err("address is invalid".to_owned());
    }
    Ok(())
}

pub fn list_address(node: String) -> Result<Vec<String>, String> {
    let wallets = Wallets::new_wallets(node).unwrap();
    Ok(wallets.list_address())
}

pub fn print_chain(node: String) -> Result<(), String> {
    let block_chain = BlockChain::new_blockchain(node);
    let chain_iter = block_chain.iter();
    for block in chain_iter {
        let mut block_table = Table::new();
        block_table.add_row(Row::new(vec![
            Cell::new("Block"),
            Cell::new("Height"),
            Cell::new("Nonce"),
            Cell::new("PrevBlock"),
            Cell::new("Pow"),
            Cell::new("timestamp"),
        ]));
        block_table.add_row(Row::new(vec![
            Cell::new(&util::encode_hex(&block.hash)),
            Cell::new(&format!("{}", &block.height)),
            Cell::new(&format!("{}", &block.nonce)),
            Cell::new(&util::encode_hex(&block.prev_block_hash)),
            Cell::new(&format!(
                "{:?}",
                ProofOfWork::new_proof_of_work(&block.clone())
                    .validate()
            )),
            Cell::new(&format!("{}", &block.timestamp)),
        ]));
        for i in 0..3 {
            println!("");
        }
        println!("Block");
        block_table.printstd();
        let tx_number = RefCell::new(1);
        &block.transactions.into_iter().for_each(|tx| {
            let (txid, in_rows, out_rows) = tx.to_string(true);
            let mut in_table = Table::new();
            let mut out_table = Table::new();
            in_rows.into_iter().for_each(|row| {
                in_table.add_row(row);
                ()
            });
            out_rows.into_iter().for_each(|row| {
                out_table.add_row(row);
                ()
            });
            {
                println!("交易{}, id:{}", tx_number.borrow(), txid);
            }
            println!("Inputs");
            in_table.printstd();
            println!("Outputs");
            out_table.printstd();
            *tx_number.borrow_mut() += 1;
        });
        if block.prev_block_hash.len() == 0 {
            break;
        }
    }
    Ok(())
}

pub fn reindex_utxo(node: String) -> Result<(), String> {
    let block_chain = Arc::new(BlockChain::new_blockchain(node));
    let utxo = UTXOSet::new(block_chain.clone());
    utxo.reindex();

    let count = utxo.count_transactions();
    info!(
        LOG,
        "Done! There are {:?} transactions in the utxo set.",
        count
    );
    println!("Done! There are {} transactions in the utxo set.", count);
    Ok(())
}

pub fn get_utxo(txid: String, node: String) -> Result<(), String> {
    let block_chain = BlockChain::new_blockchain(node);
    let db = block_chain.db.clone();
    let utxos = db.get_all_with_prefix("utxo-");
    for kv in &utxos {
        let k_txid = util::encode_hex(&kv.0);
        if k_txid == txid {
            println!("{:?}", String::from_utf8_lossy(&kv.1));
        }
    }
    Ok(())
}

pub fn get_utxos(node: String) -> Result<(), String> {
    let block_chain = BlockChain::new_blockchain(node);
    let db = block_chain.db.clone();
    let utxos = db.get_all_with_prefix("utxo-");
    for kv in &utxos {
        let k_txid = util::encode_hex(&kv.0);
        println!("{:?}", k_txid);
    }
    Ok(())
}

pub fn get_balance(address: String, node: String) -> Result<(), String> {
    if !Wallet::validate_address(address.clone()) {
        return Err("ERROR: Address is not valid".to_owned());
    }
    let block_chain = Arc::new(BlockChain::new_blockchain(node));
    let utxo = UTXOSet::new(block_chain.clone());

    let mut balance = 0;
    let pub_key_hash = util::decode_base58(address.clone());
    let pub_key_hash = &pub_key_hash[1..(pub_key_hash.len() - 4)];
    let utxos = utxo.find_utxo(pub_key_hash);
    for out in utxos {
        balance += out.value;
    }
    info!(LOG, "Balance of {}: {}", address, balance);
    Ok(())
}

pub fn get_balances(wallet_store: String, node: String) -> Result<(), String> {
    let wallets = Wallets::new_wallets(wallet_store.clone()).unwrap();
    let address = wallets.list_address();
    let block_chain = Arc::new(BlockChain::new_blockchain(node.clone()));
    let utxo = UTXOSet::new(block_chain.clone());

    address.into_iter().for_each(|addr| {
        let mut balance = 0;
        let pub_key_hash = util::decode_base58(addr.clone());
        let pub_key_hash = &pub_key_hash[1..(pub_key_hash.len() - 4)];
        let utxos = utxo.find_utxo(pub_key_hash);
        for out in utxos {
            balance += out.value;
        }
        info!(LOG, "Balance of {}: {}", addr, balance);
    });
    Ok(())
}

pub fn list_transactions(node: String) -> Result<(), String> {
    let block_chain = BlockChain::new_blockchain(node.clone());
    let block_iter = block_chain.iter();
    for block in block_iter {
        for transaction in &block.transactions {
            println!(
                "交易 {:?}, 区块为:{:?}",
                util::encode_hex(&transaction.id),
                util::encode_hex(&block.hash)
            );
        }
    }
    Ok(())
}

pub fn send(
    from: String,
    to: String,
    amount: isize,
    wallet_store: String,
    node: String,
    central_node: String,
    local_addr: String,
    mine_now: bool,
) -> Result<(), String> {
    if !Wallet::validate_address(from.clone()) {
        return Err("ERROR: From's address is not valid".to_owned());
    }
    if !Wallet::validate_address(to.clone()) {
        return Err("ERROR: To's address is not valid".to_owned());
    }
    let block_chain = Arc::new(BlockChain::new_blockchain(node.clone()));
    let utxo = UTXOSet::new(block_chain.clone());
    let tx = {
        let wallets = Wallets::new_wallets(wallet_store).unwrap();
        let from_wallet = wallets.get_wallet(from.clone()).unwrap();
        transaction::Transaction::new_utxo_transaction(&from_wallet, to.clone(), amount, &utxo)?
    };
    info!(LOG, "result: {:?}", tx.id);
    {
        let (_, in_rows, out_rows) = tx.to_string(true);
        let mut in_table = Table::new();
        let mut out_table = Table::new();
        in_rows.into_iter().for_each(
            |row| { in_table.add_row(row); },
        );
        out_rows.into_iter().for_each(
            |row| { out_table.add_row(row); },
        );
        println!("Inputs");
        in_table.printstd();
        println!("Outputs");
        out_table.printstd();
    }

    if mine_now {
        let cbtx = transaction::Transaction::new_coinbase_tx(from.clone(), "".to_owned());
        let txs = vec![cbtx, tx];
        let new_block = block_chain.clone().mine_block(&txs);
        utxo.update(&new_block);
        info!(LOG, "{:?} send {} to {:?}", from, amount, to);
        return Ok(());
    }

    let known_nodes = Arc::new(Mutex::new(vec![central_node.clone()]));
    server::send_tx(known_nodes, &central_node, &local_addr, &tx);
    info!(LOG, "{:?} send {} to {:?}", from, amount, to);
    Ok(())
}

pub fn start_server(
    node: String,
    node_role: String,
    central_node: String,
    mining_addr: String,
    addr: String,
    port: u16,
) {
    let block_chain = BlockChain::new_blockchain(node);
    let local_node = format!("{}:{}", &addr, port);
    let block_state = router::BlockState::new(
        block_chain,
        local_node.clone(),
        central_node.clone(),
        mining_addr,
    );
    let known_nodes = block_state.known_nodes.clone();
    let bc = block_state.bc.clone();
    let join = thread::spawn(move || { router::init_router(&addr, port, block_state); });

    let node_role = string_to_node_role(node_role);
    let addr_list = vec![local_node.clone()];
    match node_role {
        NodeRole::MiningNode => {
            server::send_addr(known_nodes.clone(), &central_node, addr_list);
            sync_block_tick(
                known_nodes.clone(),
                &central_node,
                "/version",
                &local_node,
                bc,
            );
        }
        NodeRole::WalletNode => {
            server::send_addr(known_nodes.clone(), &central_node, addr_list);
            sync_block_tick(
                known_nodes.clone(),
                &central_node,
                "/version",
                &local_node,
                bc,
            );
        }
        NodeRole::CentralNode => {
            if local_node.clone() != central_node.clone() {
                server::send_addr(known_nodes.clone(), &central_node, addr_list);
                sync_block_tick(
                    known_nodes.clone(),
                    &central_node,
                    "/version",
                    &local_node,
                    bc,
                );
            }
        }
    }
    join.join().unwrap();
}

enum NodeRole {
    CentralNode,
    WalletNode,
    MiningNode,
}

fn string_to_node_role(node_role: String) -> NodeRole {
    match &node_role[..] {
        "central" => NodeRole::CentralNode,
        "wallet" => NodeRole::WalletNode,
        "mining" => NodeRole::MiningNode,
        no => panic!(format!("{} is invalid node role", no)),
    }
}

fn sync_block_tick(
    known_nodes: Arc<Mutex<Vec<String>>>,
    addr: &str,
    path: &str,
    local_node: &str,
    bc: Arc<BlockChain>,
) {
    let tick = chan::tick(Duration::from_secs(3));
    server::send_version(known_nodes.clone(), addr, path, local_node, bc.clone());
    loop {
        tick.recv().unwrap();
        server::send_version(known_nodes.clone(), addr, path, local_node, bc.clone());
        sync_block_peer(known_nodes.clone(), addr, "/node/list");
    }
}

fn sync_block_peer(known_nodes: Arc<Mutex<Vec<String>>>, addr: &str, path: &str) {

    #[derive(Serialize, Deserialize, Debug, Default, Clone)]
    struct Data {
        data: Vec<String>,
        status: String,
    }
    let f: Box<Fn(Vec<u8>) + Send> = Box::new(move |data| {
        let result = serde_json::from_slice(&data.clone());
        if result.is_err() {
            error!(
                LOG,
                "sync peer nodes list error: {:?}",
                result.err().unwrap()
            );
        } else {
            let addrs: Data = result.unwrap();
            if addrs.status != "ok" {
                error!(LOG, "sync peer nodes fail, status code {}", addrs.status);
                return;
            }
            let addr_list = addrs.data.clone();
            {
                let mut known_nodes = known_nodes.lock().unwrap();
                addr_list.into_iter().for_each(|addr| {
                    let exist = known_nodes.clone().into_iter().all(|node| node != addr);
                    if exist {
                        known_nodes.push(addr);
                    }
                });
                info!(LOG, "There are {} known nodes now", known_nodes.len());
            }
        }
    });
    let mut arg = pool::DataArg::new(
        addr.to_owned(),
        path.to_owned(),
        "GET".to_owned(),
        vec![],
        &[],
    );
    arg.set_call_back(f);
    debug!(LOG, "sync peer nodes");
    pool::put_job(arg);
}
