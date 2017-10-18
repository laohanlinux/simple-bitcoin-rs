extern crate slog;
extern crate slog_term;

use super::block;
use super::transaction::*;
use super::blockchain::BlockChain;
use super::util;
use super::db::dec_key;
use super::log::*;

use std::collections::HashMap;
use std::cell::Ref;

pub struct UTXOSet<'a> {
    pub blockchain: Ref<'a, BlockChain>,
}

impl<'a> UTXOSet<'a> {
    const UTXO_BLOCK_PREFIX: &'static str = "utxo-";

    pub fn new(blockchain: Ref<BlockChain>) -> UTXOSet {
        UTXOSet { blockchain: blockchain }
    }

    pub fn find_spend_able_outputs(
        &self,
        pubkey_hash: &[u8],
        amout: isize,
    ) -> (isize, HashMap<String, Vec<isize>>) {
        let mut unspent_outs: HashMap<String, Vec<isize>> = HashMap::new();
        let mut accumulated = 0;
        let db = self.blockchain.db.borrow();

        let kvs = db.get_all_with_prefix(Self::UTXO_BLOCK_PREFIX);
        for kv in &kvs {
            let txid = util::encode_hex(&kv.0);
            println!("find_spend_able_outputs txid: {:?}", &kv.0);
            let outs = TXOutputs::deserialize_outputs(&kv.1);
            let mut idx = 0;
            for out in &*outs.outputs {
                if out.is_locked_with_key(pubkey_hash) && accumulated < amout {
                    accumulated += out.value;
                    let new_value = {
                        let value = unspent_outs.get_mut(&txid.clone());
                        value.map_or(vec![idx], |v| {
                            v.push(idx);
                            v.to_vec()
                        })
                    };
                    unspent_outs.insert(txid.clone(), new_value);
                }
                idx += 1;
            }
        }
        (accumulated, unspent_outs)
    }

    // |netenv|pub_key_hash|checksum|
    pub fn find_utxo(&self, pubkey_hash: &[u8]) -> Vec<TXOutput> {
        let mut utxos = Vec::<TXOutput>::new();
        let db = &self.blockchain.db.borrow();
        let kvs = db.get_all_with_prefix(Self::UTXO_BLOCK_PREFIX);
        if kvs.len() == 0 {
            warn!(LOG, "no utxo in blockchain({})", Self::UTXO_BLOCK_PREFIX);
        }
        for kv in &kvs {
            let outs = TXOutputs::deserialize_outputs(&kv.1);
            for out in &*outs.outputs {
                info!(LOG, "{:?}, {:?}", &pubkey_hash, &out.pub_key_hash);
                if !out.is_locked_with_key(pubkey_hash) {
                    continue;
                }
                info!(LOG, "Find a utxo {:?}", &pubkey_hash);
                utxos.push(out.clone());
            }
        }
        utxos
    }

    pub fn count_transactions(&self) -> usize {
        let db = &self.blockchain.db.borrow();
        let kvs = db.get_all_with_prefix(Self::UTXO_BLOCK_PREFIX);
        kvs.len()
    }

    pub fn reindex(&self) {
        let db = self.blockchain.db.borrow();
        let kvs = db.get_all_with_prefix(Self::UTXO_BLOCK_PREFIX);
        if kvs.len() == 0 {
            warn!(LOG, "no utxo in db");
        }
        for kv in &kvs {
            db.delete(&kv.0, Self::UTXO_BLOCK_PREFIX);
            warn!(
                LOG,
                "delete key {:?}, {:?}",
                kv.0,
                &kv.1
            );
        }

        let utxos = self.blockchain.find_utxo();
        if utxos.is_none() {
            warn!(LOG, "all output are spend");
            return;
        }

        for kv in &utxos.unwrap() {
            info!(LOG, "unspend utxo: {}", &kv.0);
            db.put_with_prefix(
                &util::decode_hex(&kv.0),
                &TXOutputs::serialize(&kv.1),
                Self::UTXO_BLOCK_PREFIX,
            );
        }
    }

    // 增加新块，新块的交易输入可能包含了当前的“未花费”输出，这些输出需要清理掉
    pub fn update(&self, block: &block::Block) {
        let db = self.blockchain.db.borrow();
        for tx in &block.transactions {
            if !tx.is_coinbase() {
                for vin in &tx.vin {
                    // store the unspend outputs
                    let mut update_outs = TXOutputs::new(vec![]);
                    let out_bytes = db.get_with_prefix(&vin.txid, Self::UTXO_BLOCK_PREFIX)
                        .unwrap();
                    let outputs = TXOutputs::deserialize_outputs(&out_bytes);
                    for out_idx in (0..outputs.outputs.len()) {
                        if out_idx != vin.vout as usize {
                            let out = outputs.outputs[out_idx].clone();
                            update_outs.outputs.push(out);
                        }
                    }
                    if update_outs.outputs.len() == 0 {
                        // the txid's outputs all spend, delete it from db
                        db.delete(&vin.txid, Self::UTXO_BLOCK_PREFIX);
                    } else {
                        // update the outputs
                        db.put_with_prefix(
                            &vin.txid,
                            &TXOutputs::serialize(&update_outs),
                            Self::UTXO_BLOCK_PREFIX,
                        );
                    }
                }
            }

            let mut new_outputs = TXOutputs::new(vec![]);
            for out in &tx.vout {
                new_outputs.outputs.push(out.clone());
            }
            db.put_with_prefix(
                &tx.id,
                &TXOutputs::serialize(&new_outputs),
                Self::UTXO_BLOCK_PREFIX,
            );
        }
    }
}
