extern crate slog;
extern crate slog_term;
extern crate prettytable;
extern crate typemap;
extern crate rocket;

use self::prettytable::Table;
use self::prettytable::row::Row;
use self::prettytable::cell::Cell;
use self::typemap::Key;

use super::util;
use super::log::*;
use super::wallets::Wallets;
use super::wallet::Wallet;
use super::blockchain::BlockChain;
use super::utxo_set::UTXOSet;
use super::proof_of_work::ProofOfWork;
use super::transaction;
use super::server;

use std::fs;
use std::cell::{Ref, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

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
    let ref_bc = RefCell::new(blockchain);
    UTXOSet::new(ref_bc.borrow()).reindex();
    info!(LOG, "utxoset reindexs successfully.");
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
        ]));
        println!("Block");
        block_table.printstd();
        let tx_number = RefCell::new(1);
        &block.transactions.into_iter().for_each(|tx| {
            let (txid, in_rows, out_rows) = tx.to_string();
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
    let block_chain = RefCell::new(BlockChain::new_blockchain(node));
    let utxo = UTXOSet::new(block_chain.borrow());
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
    let db = block_chain.db.borrow();
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
    let db = block_chain.db.borrow();
    let utxos = db.get_all_with_prefix("utxo-");
    for kv in &utxos {
        let k_txid = util::encode_hex(&kv.0);
    }
    Ok(())
}

pub fn get_balance(address: String, node: String) -> Result<(), String> {
    if !Wallet::validate_address(address.clone()) {
        return Err("ERROR: Address is not valid".to_owned());
    }
    let block_chain = RefCell::new(BlockChain::new_blockchain(node));
    let utxo = UTXOSet::new(block_chain.borrow());

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
    let block_chain = RefCell::new(BlockChain::new_blockchain(node.clone()));
    let utxo = UTXOSet::new(block_chain.borrow());

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
    mine_now: bool,
) -> Result<(), String> {
    if !Wallet::validate_address(from.clone()) {
        return Err("ERROR: From's address is not valid".to_owned());
    }
    if !Wallet::validate_address(to.clone()) {
        return Err("ERROR: To's address is not valid".to_owned());
    }
    let block_chain = Rc::new(RefCell::new(BlockChain::new_blockchain(node.clone())));
    let utxo = UTXOSet::new(block_chain.borrow());
    let result = {
        let wallets = Wallets::new_wallets(wallet_store).unwrap();
        let from_wallet = wallets.get_wallet(from.clone()).unwrap();
        transaction::Transaction::new_utxo_transaction(&from_wallet, to.clone(), amount, &utxo)?
    };
    info!(LOG, "result: {:?}", result.id);
    {
        let (txid, in_rows, out_rows) = result.to_string();
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
    let new_block = if mine_now {
        let cbtx = transaction::Transaction::new_coinbase_tx(from.clone(), "".to_owned());
        let txs = vec![cbtx, result];
        let new_block = block_chain.borrow().mine_block(&txs);
        Some(new_block)
    } else {
        None
    };
    if new_block.is_none() {
        return Err("ERROR: generate block fail".to_owned());
    }
    utxo.update(&new_block.unwrap());
    info!(LOG, "{:?} send {} to {:?}", from, amount, to);
    Ok(())
}


pub fn start_server(node: String, addr: &str, port: u32) {
    let block_chain = Arc::new(Mutex::new(BlockChain::new_blockchain(node.clone())));
    let mut config = rocket::config::Config::production().expect("cwd");
    config.set_address(addr).unwrap();
    config.set_port(port as u16);
    rocket::custom(config, true).mount("/message", routes![server::new]).launch();
}

//pub struct BC;
//impl Key for BC {
//    type Value = Arc<Mutex<BlockChain>>;
//}
//
//pub fn server(node: String, addr: &str, port: u32) {
//    let block_chain = Arc::new(Mutex::new(BlockChain::new_blockchain(node.clone())));
//
//    // add webserver
//    let mut server = sapper::SapperApp::new();
//
//    server.address(addr).port(port).init_global(Box::new(
//        move |req: &mut Request| -> SapResult<()> {
//            req.ext_mut().insert::<BC>(block_chain.clone());
//            Ok(())
//        },
//    ));
//    info!(LOG, "start a http node {}:{}", addr, port);
//    server.run_http();
//}
