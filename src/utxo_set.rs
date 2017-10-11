use std::cell::RefCell;

use super::block;
use super::transaction::*;
use super::blockchain::BlockChain;
use super::util;

use std::collections::HashMap;

pub struct UTXOSet<'a> {
    pub blockchain: &'a BlockChain,
}

impl <'a> UTXOSet <'a> {
    const UTXO_BLOCK_PREFIX: &'a str = "utxo-";
    pub fn find_spend_able_outputs(&self, pubkey_hash: &[u8], amout: isize) -> (isize, HashMap<String, Vec<isize>>) {
        let mut unspent_outs: HashMap<String, Vec<isize>> = HashMap::new();
        let mut accumulated = 0;
        let db = &self.blockchain.db;

        let kvs = db.borrow().get_all_with_prefix(Self::UTXO_BLOCK_PREFIX);
        for kv in &kvs {
            let txid = util::encode_hex(&kv.0);
            let outs = TXOutputs::deserialize_outputs(&kv.1);
            let mut idx = 0;
            for out in &*outs.outputs {
                if out.is_locked_with_key(pubkey_hash) && accumulated < amout{
                    accumulated += out.value;
                    let new_value = {
                        let value = unspent_outs.get_mut(&txid.clone());
                        value.map_or(vec![idx], |v|{
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
}
