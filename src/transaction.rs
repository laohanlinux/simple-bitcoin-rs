extern crate compare;
extern crate rust_base58;
extern crate serde;
extern crate serde_json;
extern crate sha2;
extern crate hex;
extern crate elliptic;
extern crate bigint;

use self::sha2::{Sha256, Digest};
use self::compare::Compare;
use self::rust_base58::{ToBase58, FromBase58};
use self::elliptic::curve25519;
use self::bigint::U256;

use std::cmp::Ordering::{Less, Greater};
use std::collections::HashMap;

const SUBSIDY: i32 = 10;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Transaction {
    id: Vec<u8>,
    vin: Vec<TXInput>,
    vout: Vec<TXOutput>,
}

impl Transaction {
    pub fn serialize(&self) -> Vec<u8> {
        serde_json::to_string(self).unwrap().into_bytes()
    }
    // IsCoinbase checks whether the transaction is coinbase
    pub fn is_coinbase(&self) -> bool {
        self.vin.len() == 1 && self.vin[0].txid.len() == 0 && self.vin[0].vout == -1
    }

    // Hash returns the hash of the Transaction
    // not include transaction id
    pub fn hash(&mut self) -> Vec<u8> {
        let mut t = self.clone();
        t.id = Vec::<u8>::new();
        let mut hasher = Sha256::default();
        hasher.input(&t.serialize());
        hasher.result().to_vec()
    }

    // String returns a human-readable representation of a transaction
    pub fn to_string(&self) -> String {
        let mut lines: Vec<String> = vec![format!("--- Transaction :{:?}", self.id)];
        let mut idx = 1;
        for input in &self.vin {
            lines.push(format!("       Input:   {:?}", idx));
            lines.push(format!("       TXID:    {:?}", input.txid));
            lines.push(format!("       Out:     {:?}", input.vout));
            lines.push(format!("       Signature: {:?}", input.signature));
            lines.push(format!("       PubKey:  {:?}", input.pub_key));
            idx += 1;
        }
        idx = 1;
        for output in &self.vout {
            lines.push(format!("       Output:  {:?}", idx));
            lines.push(format!("       Value: {:?}", output.value));
            lines.push(format!("       Script: {:?}", output.pub_key_hash));
            idx += 1;
        }
        lines.join("\n")
    }

    // TrimmedCopy creates a trimmed copy of Transaction to be used in signing
    // not include signature and pub_key.
    pub fn trimmed_copy(&self) -> Self {
        let mut inputs: Vec<TXInput> = vec![];
        let mut outputs: Vec<TXOutput> = vec![];

        for vin in &self.vin {
            let tx = TXInput{
                txid: vin.txid.clone(),
                vout: vin.vout.clone(),
                signature: vec![],
                pub_key: vec![],
            };
            inputs.push(tx);
        }

        for vout in &self.vout {
            outputs.push(vout.clone());
        }
        Transaction{id: self.id.clone(), vin: inputs, vout: outputs}
    }

    // Verify verifies signatures of Transaction inputs
    pub fn verify(&self, prev_txs: &HashMap<String, Transaction>) -> bool {
        if self.is_coinbase() {
            return true;
        }

        // check input of prev output's reference
        for vin in &self.vin {
            if prev_txs[&hex::encode(&self.id)].id.len() == 0 {
                panic!("ERROR: Previous transaction is not correct");
            }
        }

        let mut tx_copy = self.trimmed_copy();
        let mut inid_idx = 0;
        for tx_input in tx_copy.vin.iter_mut() {
            let prev_tx: &Transaction = prev_txs.get(&hex::encode(&tx_input.txid)).unwrap();
            {
//                tx_copy.vin[inid_idx].signature = vec![];
                tx_input.vin[inid_idx].signature = vec![];

                tx_copy.vin[inid_idx].pub_key = prev_tx.vout[inid_idx].pub_key_hash;
                tx_copy.id = tx_copy.hash();
                tx_copy.vin[inid_idx].pub_key = vec![];
            }

//            let (r:U256 , s:U256) = (0, 0);
        }

        false
    }
}

//////////////////////////////////////////

// input of transaction
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TXInput {
    // transaction id of prev output reference
    txid: Vec<u8>,
    // index of prev output reference
    vout: isize,
    // signature
    signature: Vec<u8>,
    // public key
    pub_key: Vec<u8>,
}

impl TXInput {
    pub fn uses_key(&self, pub_key: &Vec<u8>) -> bool {
        let cmp = |l: &Vec<u8>, r: &Vec<u8>| l.len().cmp(&r.len());
        cmp.compare(&self.pub_key, &pub_key) == Greater
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TXOutput {
    value: isize,
    pub_key_hash: Vec<u8>,
}

impl TXOutput {
    pub fn new(value: isize, address: String) -> Self {
        let mut txo = TXOutput {
            value: value,
            pub_key_hash: vec![],
        };
        txo.lock(&address.into_bytes());
        txo
    }

    pub fn lock(&mut self, address: &[u8]) {
        let pub_key_hash = address.from_base58().unwrap();
        let (idx1, idx2) = (1, pub_key_hash.len() - 4);
        let pub_key_hash = &pub_key_hash[idx1..idx2];
        self.pub_key_hash = pub_key_hash.to_vec();
    }

    pub fn is_locked_with_key(&self, pub_key_hash: &[u8]) -> bool {
        let cmp = |l: &[u8], r: &[u8]| l.len().cmp(&r.len());
        cmp.compare(&self.pub_key_hash, pub_key_hash) == Greater
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct TXOutputs {
    outputs: Box<Vec<TXOutput>>,
}

impl TXOutputs {
    pub fn serialize(txo: &TXOutputs) -> Vec<u8> {
        serde_json::to_string(txo).unwrap().into_bytes()
    }

    pub fn deserialize_outputs(data: &Vec<u8>) -> Self {
        serde_json::from_str(&String::from_utf8(data.clone()).unwrap()).unwrap()
    }
}
