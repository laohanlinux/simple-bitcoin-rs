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

    // HashMap =>  [txid, Vec![out'idx1, out'idx2]]
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
            warn!(LOG, "序列化的交易id: {:?}", &txid);
            let outs = TXOutputs::deserialize_outputs(&kv.1);
            for (out_idx, out) in &*outs.outputs {
                if out.is_locked_with_key(pubkey_hash) && accumulated < amout {
                    debug!(
                        LOG,
                        "解锁:{:?} - 交易:{:?} - 索引:{:?} - 可用资产: {:?}",
                        util::encode_hex(pubkey_hash),
                        &txid,
                        out_idx,
                        out.value
                    );
                    accumulated += out.value;
                    unspent_outs.entry(txid.clone()).or_insert(vec![]).push(
                        *out_idx,
                    );
                }
            }
        }
        for (k, v) in &unspent_outs {
            debug!(LOG, "校验数据, 交易{:?}, 索引: {:?}", k, v);
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
            warn!(
                LOG,
                "find_utxo 序列化的交易id {:?}",
                util::encode_hex(&kv.0)
            );
            let outs = TXOutputs::deserialize_outputs(&kv.1);
            for (out_idx, out) in &*outs.outputs {
                if !out.is_locked_with_key(pubkey_hash) {
                    info!(
                        LOG,
                        "skip the pubkey_hash: {:?}",
                        util::encode_hex(&pubkey_hash)
                    );
                    continue;
                }
                info!(
                    LOG,
                    "Find a utxo =============> {:?}",
                    util::encode_hex(&pubkey_hash)
                );
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
            warn!(LOG, "delete key {:?}, {:?}", kv.0, &kv.1);
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
                    let mut update_outs = TXOutputs::new(HashMap::new());
                    println!(
                        "当前的输入交易id为:{:?} - {:?}， 区块为:{:?}",
                        &util::encode_hex(&vin.txid),
                        &vin.vout,
                        &util::encode_hex(&block.hash)
                    );
                    let out_bytes = db.get_with_prefix(&vin.txid, Self::UTXO_BLOCK_PREFIX)
                        .unwrap();
                    let outputs = TXOutputs::deserialize_outputs(&out_bytes);

                    for (out_idx, out) in &*outputs.outputs {
                        if *out_idx != vin.vout {
                            update_outs.outputs.insert(*out_idx, out.clone());
                        }
                    }
                    if update_outs.outputs.len() == 0 {
                        // the txid's outputs all spend, delete it from db
                        debug!(LOG, "删除旧的utxo {}", util::encode_hex(&vin.txid));
                        db.delete(&vin.txid, Self::UTXO_BLOCK_PREFIX);
                    } else {
                        // update the outputs
                        debug!(LOG, "更新utxo {}", util::encode_hex(&vin.txid));
                        db.put_with_prefix(
                            &vin.txid,
                            &TXOutputs::serialize(&update_outs),
                            Self::UTXO_BLOCK_PREFIX,
                        );
                    }
                }
            }

            let mut new_outputs = TXOutputs::new(HashMap::new());
            let mut out_idx = 0;
            for out in &*tx.vout {
                new_outputs.outputs.insert(out_idx, out.clone());
                out_idx += 1;
            }
            debug!(LOG, "增加新的UTXO {}", util::encode_hex(&tx.id));
            db.put_with_prefix(
                &tx.id,
                &TXOutputs::serialize(&new_outputs),
                Self::UTXO_BLOCK_PREFIX,
            );
        }
    }
}
