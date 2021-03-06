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

pub fn create_wallet(node: &str, del_old: bool) {
    if del_old {
        fs::remove_file(node).unwrap();
    }
    let wallets = Wallets::new().unwrap();
    wallets.save_to_file(node);
    info!(
        LOG,
        "All your wallet  address:",
    );
    let address = wallets.list_address();
    address.into_iter().fold(0, |acc, addr| {
        info!(LOG, "addr[{:?}]=> {:?}", acc, addr);
        acc + 1
    });
}

pub fn add_wallet(node: &str) {
    let mut wallets = Wallets::new_wallets(node.to_string()).unwrap();
    let new_address = wallets.create_wallet();
    fs::remove_file(&node).unwrap();
    wallets.save_to_file(node);
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

pub fn create_blockchain(address: &str, node: &str) -> Result<(), String> {
    if !Wallet::validate_address(address.to_string()) {
        return Err("address is invalid".to_owned());
    }
    let blockchain = BlockChain::create_blockchain(address.to_string(), node.to_string());
    info!(LOG, "block chain disk data create successfully.");
    let ref_bc = Arc::new(blockchain);
    UTXOSet::new(Arc::clone(&ref_bc)).reindex();
    let last_hash = ref_bc.last_block_hash();
    info!(LOG, "utxoset reindexs successfully.");
    info!(LOG, "genius block {}", last_hash);
    Ok(())
}

pub fn address_check(address: &str) -> Result<(), String> {
    if !Wallet::validate_address(address.to_string()) {
        return Err("address is invalid".to_owned());
    }
    Ok(())
}

pub fn list_address(node: &str) -> Result<Vec<String>, String> {
    let wallets = Wallets::new_wallets(node.to_string()).unwrap();
    Ok(wallets.list_address())
}

pub fn download_chain(node: &str) -> Result<(), String> {
    let block_chain = BlockChain::new_blockchain(node.to_string());
    let bcs = block_chain.all_blocks();
    Ok(())
}

pub fn print_chain(node: &str) -> Result<(), String> {
    let block_chain = BlockChain::new_blockchain(node.to_string());
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
        (0..3).for_each(|_| println!());
        println!("Block");
        block_table.printstd();
        let tx_number = RefCell::new(1);
        block.transactions.into_iter().for_each(|tx| {
            let (txid, in_rows, out_rows) = tx.to_string(true);
            let mut in_table = Table::new();
            let mut out_table = Table::new();
            in_rows.into_iter().for_each(
                |row| { in_table.add_row(row); },
            );
            out_rows.into_iter().for_each(
                |row| { out_table.add_row(row); },
            );
            {
                println!("交易{}, id:{}", tx_number.borrow(), txid);
            }
            println!("Inputs");
            in_table.printstd();
            println!("Outputs");
            out_table.printstd();
            *tx_number.borrow_mut() += 1;
        });
        if block.prev_block_hash.is_empty() {
            break;
        }
    }
    Ok(())
}

pub fn reindex_utxo(node: &str) -> Result<(), String> {
    let block_chain = Arc::new(BlockChain::new_blockchain(node.to_string()));
    let utxo = UTXOSet::new(block_chain);
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

pub fn get_utxo(txid: &str, node: &str) -> Result<(), String> {
    let block_chain = BlockChain::new_blockchain(node.to_string());
    let utxos = block_chain.db.get_all_with_prefix("utxo-");
    utxos
        .into_iter()
        .filter(|kv| util::encode_hex(&kv.0) == txid)
        .for_each(|kv| println!("{:?}", String::from_utf8_lossy(&kv.1)));
    Ok(())
}

pub fn get_utxos(node: &str) -> Result<(), String> {
    let block_chain = BlockChain::new_blockchain(node.to_string());
    let utxos = block_chain.db.get_all_with_prefix("utxo-");
    utxos.into_iter().for_each(|kv| {
        println!("{:?}", util::encode_hex(&kv.0))
    });
    Ok(())
}

pub fn get_balance(address: &str, node: &str) -> Result<(), String> {
    if !Wallet::validate_address(address.to_string()) {
        return Err("ERROR: Address is not valid".to_owned());
    }
    let block_chain = Arc::new(BlockChain::new_blockchain(node.to_string()));
    let utxo = UTXOSet::new(Arc::clone(&block_chain));

    let mut balance = 0;
    let pub_key_hash = util::decode_base58(address.to_string());
    let pub_key_hash = &pub_key_hash[1..(pub_key_hash.len() - 4)];
    let utxos = utxo.find_utxo(pub_key_hash);
    for out in utxos {
        balance += out.value;
    }
    info!(LOG, "Balance of {}: {}", address, balance);
    Ok(())
}

pub fn get_balances(wallet_store: &str, node: &str) -> Result<(), String> {
    let wallets = Wallets::new_wallets(wallet_store.to_string()).unwrap();
    let address = wallets.list_address();
    let block_chain = Arc::new(BlockChain::new_blockchain(node.to_string()));
    let utxo = UTXOSet::new(Arc::clone(&block_chain));

    address.into_iter().for_each(|addr| {
        let mut balance = 0;
        let pub_key_hash = util::decode_base58(addr.to_string());
        let pub_key_hash = &pub_key_hash[1..(pub_key_hash.len() - 4)];
        let utxos = utxo.find_utxo(pub_key_hash);
        for out in utxos {
            balance += out.value;
        }
        info!(LOG, "Balance of {}: {}", addr, balance);
    });
    Ok(())
}

pub fn list_transactions(node: &str) -> Result<(), String> {
    let block_chain = BlockChain::new_blockchain(node.to_string());
    let block_iter = block_chain.iter();
    block_iter.for_each(|block| {
        block.transactions.clone().into_iter().for_each(|ts| {
            info!(
                LOG,
                "transaction: {:?}, block: {:?}",
                util::encode_hex(&ts.id),
                util::encode_hex(&block.hash),
            );
        });
    });
    Ok(())
}

pub fn send(
    from: &str,
    to: &str,
    amount: isize,
    wallet_store: String,
    node: &str,
    central_node: &str,
    local_addr: &str,
    mine_now: bool,
) -> Result<(), String> {
    if !Wallet::validate_address(from.to_owned()) {
        return Err("ERROR: From's address is not valid".to_owned());
    }
    if !Wallet::validate_address(to.to_string()) {
        return Err("ERROR: To's address is not valid".to_owned());
    }
    let block_chain = Arc::new(BlockChain::new_blockchain(node.to_string()));
    let utxo = UTXOSet::new(Arc::clone(&block_chain));
    let tx = {
        let wallets = Wallets::new_wallets(wallet_store).unwrap();
        let from_wallet = wallets.get_wallet(from.to_owned()).unwrap();
        transaction::Transaction::new_utxo_transaction(
            from_wallet,
            to.to_string(),
            amount,
            &utxo,
            None,
        )?
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
        let cbtx = transaction::Transaction::new_coinbase_tx(from.to_string(), "".to_owned());
        let txs = vec![cbtx, tx];
        let new_block = &block_chain.mine_block(&txs).unwrap();
        utxo.update(new_block);
        info!(LOG, "{:?} send {} to {:?}", from, amount, to);
        return Ok(());
    }

    let known_nodes = Arc::new(Mutex::new(vec![central_node.to_string()]));
    server::send_tx(&known_nodes, central_node, local_addr, &tx);
    info!(LOG, "{:?} send {} to {:?}", from, amount, to);
    Ok(())
}

pub fn start_server(
    node: String,
    node_role: &str,
    central_node: &str,
    mining_addr: &str,
    addr: String,
    port: u16,
) {
    let block_chain = BlockChain::new_blockchain(node);
    let local_node = format!("{}:{}", &addr, port);
    let block_state = router::BlockState::new(
        block_chain,
        local_node.clone(),
        central_node,
        mining_addr.to_string(),
    );
    let known_nodes = Arc::clone(&block_state.known_nodes);
    let bc = Arc::clone(&block_state.bc.lock().unwrap().block_chain());
    let join = thread::spawn(move || { router::init_router(&addr, port, block_state); });

    let node_role = string_to_node_role(node_role);
    let addr_list = vec![local_node.clone()];

    let bc = Arc::clone(&bc);
    match node_role {
        NodeRole::MiningNode => {
            info!(
                LOG,
                "start as mining role, mining pub address is {}",
                mining_addr
            );
            server::send_addr(&known_nodes, central_node, addr_list);
            sync_block_tick(&known_nodes, central_node, "/version", &local_node, &bc);
        }
        NodeRole::WalletNode => {
            info!(LOG, "start as wallet role");
            server::send_addr(&known_nodes, central_node, addr_list);
            sync_block_tick(&known_nodes, central_node, "/version", &local_node, &bc);
        }
        NodeRole::CentralNode => {
            info!(LOG, "start as central role");
            if local_node != central_node {
                server::send_addr(&known_nodes, central_node, addr_list);
                sync_block_tick(&known_nodes, central_node, "/version", &local_node, &bc);
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

fn string_to_node_role(node_role: &str) -> NodeRole {
    match &node_role[..] {
        "central" => NodeRole::CentralNode,
        "wallet" => NodeRole::WalletNode,
        "mining" => NodeRole::MiningNode,
        no => panic!(format!("{} is invalid node role", no)),
    }
}

fn sync_block_tick(
    known_nodes: &Arc<Mutex<Vec<String>>>,
    addr: &str,
    path: &str,
    local_node: &str,
    bc: &Arc<BlockChain>,
) {
    let tick = chan::tick(Duration::from_secs(3));
    server::send_version(known_nodes, addr, path, local_node, bc);
    loop {
        tick.recv().unwrap();
        server::send_version(known_nodes, addr, path, local_node, bc);
        sync_block_peer(Arc::clone(known_nodes), addr, "/node/list");
    }
}

fn sync_block_peer(known_nodes: Arc<Mutex<Vec<String>>>, addr: &str, path: &str) {
    #[derive(Serialize, Deserialize, Debug, Default, Clone)]
    struct Data {
        data: Vec<String>,
        status: String,
    }
    let f: Box<Fn(Vec<u8>) + Send> = Box::new(move |data| {
        let result = serde_json::from_slice(&data.clone()).and_then(|addrs: Data| {
            if addrs.status != "ok" {
                error!(LOG, "sync peer nodes fail, status code {}", addrs.status);
                return Ok(());
            }
            let mut known_nodes = known_nodes.lock().unwrap();
            addrs.data.into_iter().for_each(|addr| {
                let exist = known_nodes.clone().into_iter().all(|node| node != addr);
                if exist {
                    known_nodes.push(addr);
                }
            });
            Ok(())
        });
        if let Err(e) = result {
            error!(LOG, "sync peer nodes list error: {:?}", e);
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
    pool::put_job(arg);
}
