extern crate compare;
extern crate rust_base58;
extern crate serde;
extern crate serde_json;
extern crate sha2;
extern crate hex;
extern crate bigint;
extern crate secp256k1;
extern crate rand;
extern crate prettytable;
extern crate slog;
extern crate slog_term;

use self::sha2::{Sha256, Digest};
use self::secp256k1::Message;
use self::secp256k1::key::SecretKey;
use self::rand::{Rng, thread_rng};
use self::prettytable::row::Row;
use self::prettytable::cell::Cell;

use super::util;
use super::log::*;
use super::wallet::{Wallet, ADDRESS_CHECKSUM_LEN};
use std::collections::HashMap;
use super::utxo_set::UTXOSet;

const SUBSIDY: isize = 10;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Transaction {
    pub id: Vec<u8>,
    pub vin: Vec<TXInput>, // 索引为vec的索引
    pub vout: Vec<TXOutput>, // 索引为vec的索引
}

impl Transaction {
    // creates a new coinbase transaction
    pub fn new_coinbase_tx(to: String, data: String) -> Transaction {
        let data = if data.len() == 0 {
            let mut randon_msg = [0u8; 32];
            thread_rng().fill_bytes(&mut randon_msg);
            util::encode_hex(&randon_msg)
        } else {
            data
        };

        let txin = TXInput::new(vec![], -1, vec![], data.into_bytes());
        let txout = TXOutput::new(SUBSIDY, to);
        let mut tx = Transaction {
            id: vec![],
            vin: vec![txin],
            vout: vec![txout],
        };
        let hash = tx.hash();
        tx.id = hash;
        tx
    }

    pub fn new_utxo_transaction(
        wallet: &Wallet,
        to: String,
        amount: isize,
        utxoset: &UTXOSet,
    ) -> Result<Transaction, String> {
        let (mut inputs, mut outputs) = (vec![], vec![]);
        let pub_key = util::public_key_to_vec(&wallet.public_key, false);
        let pub_key_hash = Wallet::hash_pubkey(&pub_key);
        // find the account unspend utxo from utxoset
        let (acc, valid_outputs) = utxoset.find_spend_able_outputs(&pub_key_hash, amount);
        if acc < amount {
            return Err("ERROR: Not enough founds".to_owned());
        }
        println!("================================================");
        debug!(LOG, "找到的可被消费的输出为:");
        // Build a list of inputs
        for kv in &valid_outputs {
            let txid = util::decode_hex(&kv.0);
            for out in kv.1 {
                debug!(LOG, "交易的ID {:?}, 交易的索引{:?} ", &kv.0, out);
                let input = TXInput::new(txid.clone(), *out, vec![], pub_key.clone());
                inputs.push(input);
            }
        }
        // Build a list of outputs
        outputs.push(TXOutput::new(amount, to));
        if acc > amount {
            outputs.push(TXOutput::new(acc - amount, wallet.get_address()));
        }

        let mut tx = Transaction {
            id: vec![],
            vin: inputs,
            vout: outputs,
        };
        let txid = tx.hash();
        tx.id = txid;
        debug!(
            LOG,
            "此次生产的交易id {:?}",
            util::encode_hex(&tx.id)
        );
        utxoset.blockchain.sign_transaction(
            &mut tx,
            &wallet.secret_key,
        );
        println!("================================================");
        Ok(tx)
    }

    // TODO add
    pub fn deserialize_transaction(data: &Vec<u8>) -> Transaction {
        // serde_json::from_str(&String::from_utf8(data.clone()).unwrap()).unwrap()
        serde_json::from_slice(data).unwrap()
    }

    pub fn serialize(&self) -> Vec<u8> {
        // serde_json::to_string(self).unwrap().into_bytes()
        serde_json::to_vec(self).unwrap()
    }
    // IsCoinbase checks whether the transaction is coinbase
    pub fn is_coinbase(&self) -> bool {
        self.vin.len() == 1 && self.vin[0].txid.len() == 0 && self.vin[0].vout == -1
    }

    // Hash returns the hash of the Transaction
    // not include transaction id
    pub fn hash(&self) -> Vec<u8> {
        let mut t = self.clone();
        t.id = Vec::<u8>::new();
        let mut hasher = Sha256::default();
        hasher.input(&t.serialize());
        hasher.result().to_vec()
    }

    pub fn sign(&mut self, secret_key: &SecretKey, prev_txs: &HashMap<String, Transaction>) {
        if self.is_coinbase() {
            return;
        }

        // check input wether reference some pre block output
        for tx_input in self.vin.iter() {
            if prev_txs.get(&hex::encode(&tx_input.txid)).is_none() {
                panic!("ERROR: Previous transaction is not correct");
            }
        }

        let mut tx_copy = self.trimmed_copy();
        let mut sign_vec = Vec::new();
        let mut inid_idx = 0;
        for tx_input in self.vin.iter() {
            let prev_tx: &Transaction = prev_txs.get(&hex::encode(&tx_input.txid)).unwrap();
            // reset signation
            tx_copy.vin[inid_idx].signature = vec![];
            // set reference's output public key
            tx_copy.vin[inid_idx].pub_key = prev_tx.vout[inid_idx].pub_key_hash.clone();

            let origin_data_to_sign = util::packet_sign_content(&tx_copy);
            let origin_data_to_sign = util::double_sha256(origin_data_to_sign);
            let data_to_sign = &Message::from_slice(&origin_data_to_sign).unwrap();
            let signature = util::sign(data_to_sign, secret_key);
            sign_vec.push(signature);

            // reset tx_copy's public key
            tx_copy.vin[inid_idx].pub_key = vec![];
            inid_idx += 1;
        }
        // update signatures, notic, we not set input's public key,
        // as say, every input's public key is nil
        inid_idx = 0;
        for tx_input in self.vin.iter_mut() {
            tx_input.signature = sign_vec[inid_idx].clone();
            inid_idx += 1;
        }
    }

    // String returns a human-readable representation of a transaction
    pub fn to_string(&self) -> (String, Vec<Row>, Vec<Row>) {
        let txid = util::encode_hex(&self.id);
        let input_row = Row::new(vec![
            Cell::new("in's idx"),
            Cell::new("in's txid"),
            Cell::new("in's ref out's idx"),
            Cell::new("signature"),
            Cell::new("PubKey"),
        ]);
        let output_row = Row::new(vec![
            Cell::new("out's idx"),
            Cell::new("out's value"),
            Cell::new("out's script"),
        ]);

        let mut input_records = vec![input_row];
        let mut output_records = vec![output_row];

        let mut idx = 0;
        for input in &self.vin {
            let input_record = vec![
                Cell::new(&format!("{}", idx)),
                Cell::new(&util::encode_hex(&input.txid)),
                Cell::new(&format!("{}", input.vout)),
                Cell::new(&util::encode_hex(&input.signature)),
                Cell::new(&util::encode_hex(&input.pub_key)),
            ];
            input_records.push(Row::new(input_record));
            idx += 1;
        }
        idx = 0;
        for output in &self.vout {
            let output_record = vec![
                Cell::new(&format!("{}", idx)),
                Cell::new(&format!("{:?}", output.value)),
                Cell::new(&util::encode_hex(&output.pub_key_hash)),
            ];
            output_records.push(Row::new(output_record));
            idx += 1;
        }
        (txid, input_records, output_records)
    }

    // TrimmedCopy creates a trimmed copy of Transaction to be used in signing
    // not include signature and pub_key.
    pub fn trimmed_copy(&self) -> Self {
        let mut inputs: Vec<TXInput> = vec![];
        let mut outputs: Vec<TXOutput> = vec![];

        for vin in &self.vin {
            let tx = TXInput {
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
        Transaction {
            id: self.id.clone(),
            vin: inputs,
            vout: outputs,
        }
    }

    #[inline]
    // Verify verifies signatures of Transaction inputs
    // tx_input = |txid|vout|sig|pkey| ==> |txid = 0| vout| sig = "" | pkey = reference pkey|
    // ==> sign(vout, pkey)
    pub fn verify(&self, prev_txs: &HashMap<String, Transaction>) -> bool {
        if self.is_coinbase() {
            return true;
        }

        // check input of prev output's reference
        for vin in &self.vin {
            if prev_txs[&hex::encode(&vin.txid)].id.len() == 0 {
                panic!("ERROR: Previous transaction is not correct");
            }
        }

        let tx_copy = &mut self.trimmed_copy();
        let mut inid_idx = 0;

        // TODO
        for tx_input in self.vin.iter() {
            let prev_tx: &Transaction = prev_txs.get(&hex::encode(&tx_input.txid)).unwrap();
            tx_copy.vin[inid_idx].signature = vec![];
            tx_copy.vin[inid_idx].pub_key = prev_tx.vout[inid_idx].pub_key_hash.clone();

            println!("public_key {:?}", &tx_input.pub_key);

            let origin_data_to_sign = util::packet_sign_content(&tx_copy);
            let verify = util::verify(&tx_input.pub_key, &tx_input.signature, origin_data_to_sign);
            if verify {
                return verify;
            }

            tx_copy.vin[inid_idx].pub_key = vec![];
            inid_idx += 1;
        }
        true
    }
}

//////////////////////////////////////////

// input of transaction
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TXInput {
    // transaction id of prev output reference
    pub txid: Vec<u8>,
    // index of prev output reference
    pub vout: isize,
    // signature
    signature: Vec<u8>,
    // public key, it is a ripemd160 format pub key
    pub pub_key: Vec<u8>,
}

impl TXInput {
    pub fn new(txid: Vec<u8>, vout: isize, signature: Vec<u8>, pub_key: Vec<u8>) -> TXInput {
        TXInput {
            txid: txid,
            vout: vout,
            signature: signature,
            pub_key: pub_key,
        }
    }
    pub fn uses_key(&self, pub_key: &Vec<u8>) -> bool {
        util::compare_slice_u8(&self.pub_key, &pub_key)
    }
}

// TODO add signature script instead of pub_key_hash
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct TXOutput {
    // value is the output's source counts
    pub value: isize,
    pub pub_key_hash: Vec<u8>,
}

impl TXOutput {
    pub fn new(value: isize, address: String) -> Self {
        let mut txo = TXOutput {
            value: value,
            ..Default::default()
        };
        txo.lock(address);
        txo
    }

    pub fn lock(&mut self, address: String) {
        let pub_key_hash = util::decode_base58(address);
        let (idx1, idx2) = (1, pub_key_hash.len() - ADDRESS_CHECKSUM_LEN);
        let pub_key_hash = &pub_key_hash[idx1..idx2];
        self.pub_key_hash = pub_key_hash.to_vec();
    }

    pub fn is_locked_with_key(&self, pub_key_hash: &[u8]) -> bool {
        util::compare_slice_u8(&self.pub_key_hash, pub_key_hash)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TXOutputs {
    pub outputs: Box<HashMap<isize, TXOutput>>,
}

impl TXOutputs {
    pub fn new(outputs: HashMap<isize, TXOutput>) -> TXOutputs {
        TXOutputs { outputs: Box::new(outputs) }
    }
    // TODO
    pub fn serialize(txo: &TXOutputs) -> Vec<u8> {
        serde_json::to_vec(txo).unwrap()
    }

    // TODO
    pub fn deserialize_outputs(data: &Vec<u8>) -> TXOutputs {
        println!("deserialize=> {:?}", String::from_utf8_lossy(data));
        serde_json::from_slice(data).unwrap()
    }
}


#[cfg(test)]

mod tests {
    use std::collections::HashMap;
    const addr: &str = "17tQE4NbkiTroRwCeqEQF4Y9yVFBGLpL59";
    #[test]
    fn coinbase_trasaction() {
        let value = 1000;
        let coin_base = super::TXOutput::new(value, addr.to_owned());
        let out_idx = 0;
        let mut outputs = HashMap::new();
        outputs.insert(out_idx, coin_base);
        let outputs = super::TXOutputs::new(outputs);
        
        // deserialize, serialize 
        {
            let ser = super::TXOutputs::serialize(&outputs);
            let expect_outputs = super::TXOutputs::deserialize_outputs(&ser);
            assert_eq!(expect_outputs.outputs.len(), outputs.outputs.len());
            // assert_eq!(expect_outputs.outputs.entry(, );
        }
    }


    #[test]
    fn txoutputs() {

    }
}