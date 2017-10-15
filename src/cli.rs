extern crate slog;
extern crate slog_term;

use super::util;
use super::log::*;
use super::wallets::Wallets;
use super::wallet::Wallet;
use super::blockchain::BlockChain;
use super::utxo_set::UTXOSet;
use super::proof_of_work::ProofOfWork;

use std::fs;

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

pub fn open_wallet(node: String) {
    let wallets = Wallets::new_wallets(node).unwrap();
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

pub fn create_blockchain(address: String, node: String) -> Result<(), String> {
    if !Wallet::validate_address(address.clone()) {
        return Err("address is invalid".to_owned());
    }
    let blockchain = BlockChain::create_blockchain(address, node);
    info!(LOG, "block chain disk data create successfully.");
    UTXOSet::new(&blockchain).reindex();
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
        println!("==========Block {:?}==========", util::encode_hex(&block.hash));
        println!("Height: {}", block.height);
        println!("Prev block: {:?}", util::encode_hex(&block.prev_block_hash));
        let block_clone = block.clone();
        let pow = ProofOfWork::new_proof_of_work(&block_clone);
        println!("Pow: {}", pow.validate());
        &block.transactions.into_iter().for_each(|tx| {
            let tx_vec = tx.to_string().clone();
            tx_vec.split(|v|{format!("{}", v) == "\n"}).for_each(|c|{println!("{}", c)});
        });
        if block.prev_block_hash.len() == 0 {
            break
        }
    }
    Ok(())
}

pub fn reindex_utxo(node: String) -> Result<(), String> {
    let block_chain = BlockChain::new_blockchain(node);
    let utxo = UTXOSet::new(&block_chain);
    utxo.reindex();

    let count = utxo.count_transactions();
    info!(LOG, "Done! There are {:?} transactions in the utxo set.", count);
    println!("Done! There are {} transactions in the utxo set.", count);
    Ok(())
}

pub fn get_balance(address: String, node: String) -> Result<(), String> {
    if !Wallet::validate_address(address.clone()) {
        return Err("ERROR: Address is not valid".to_owned());
    }
    let block_chain = BlockChain::new_blockchain(node);
    let utxo = UTXOSet::new(&block_chain);

    let mut balance = 0;
    let pub_key_hash = util::decode_base58(address.clone());
    let pub_key_hash = &pub_key_hash[1..(pub_key_hash.len() - 4)];
    let utxos = utxo.find_utxo(pub_key_hash);
    info!(LOG, "pub_key_has {:?}", pub_key_hash);
    for out in utxos {
        balance += out.value;
    }

    info!(LOG, "Balance of {}: {}", address, balance);
    println!("Balance of {}: {}", address, balance);
    Ok(())
}