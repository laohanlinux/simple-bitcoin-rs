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
extern crate time;

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
         //   lock_time: time::get_time().sec as u32,
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
        spend_utxos: Option<HashMap<String, Vec<isize>>>,
    ) -> Result<Transaction, String> {
        let (mut inputs, mut outputs) = (vec![], vec![]);
        let pub_key = util::public_key_to_vec(&wallet.public_key, false);
        let pub_key_hash = Wallet::hash_pubkey(&pub_key);
        // find the account unspend utxo from utxoset
        let (acc, valid_outputs) =
            utxoset.find_spend_able_outputs(&pub_key_hash, amount, spend_utxos);
        if acc < amount {
            return Err("ERROR: Not enough founds".to_owned());
        }

        // Build a list of inputs
        valid_outputs.iter().for_each(|kv| {
            kv.1.iter().for_each(|out| {
                let input = TXInput::new(util::decode_hex(&kv.0), *out, vec![], pub_key.clone());
                inputs.push(input);
            });
        });
        /*for kv in &valid_outputs {
            let txid = util::decode_hex(&kv.0);
            for out in kv.1 {
                let input = TXInput::new(txid.clone(), *out, vec![], pub_key.clone());
                inputs.push(input);
            }
        }*/
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
        let res = utxoset.blockchain.sign_transaction(
            &mut tx,
            &wallet.secret_key,
        );
        res.map(|_| tx)
    }

    // TODO add
    pub fn deserialize_transaction(data: &[u8]) -> Transaction {
        serde_json::from_slice(data).unwrap()
    }

    pub fn serialize(&self) -> Vec<u8> {
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

    pub fn sign(&mut self, secret_key: &SecretKey, prev_txs: &HashMap<isize, Transaction>) {
        if self.is_coinbase() {
            return;
        }

        // check input wether reference some pre block output
        self.vin.iter().fold(0, |acc, _| {
            assert!(prev_txs.get(&acc).is_some());
            acc + 1
        });

        let mut tx_copy = self.trimmed_copy();
        let mut sign_vec = Vec::new();

        self.vin.iter().fold(0, |acc, tx_input| {
            let prev_tx: &Transaction = prev_txs.get(&acc).unwrap();
            // reset signation
            tx_copy.vin[acc as usize].signature = vec![];
            // set reference's output public key
            tx_copy.vin[acc as usize].pub_key =
                prev_tx.vout[tx_input.vout as usize].pub_key_hash.clone();

            let origin_data_to_sign = util::packet_sign_content(&tx_copy);
            let origin_data_to_sign = util::double_sha256(origin_data_to_sign);
            let data_to_sign = &Message::from_slice(&origin_data_to_sign).unwrap();
            let signature = util::sign(data_to_sign, secret_key);
            sign_vec.push(signature);

            // reset tx_copy's public key
            tx_copy.vin[acc as usize].pub_key = vec![];
            acc + 1
        });

        // update signatures, notic, we not set input's public key,
        // as say, every input's public key is nil
        self.vin.iter_mut().fold(0, |acc, tx_input| {
            tx_input.signature = sign_vec[acc].clone();
            acc + 1
        });
    }

    // String returns a human-readable representation of a transaction
    pub fn to_string(&self, compress: bool) -> (String, Vec<Row>, Vec<Row>) {
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

        for (idx, input) in self.vin.iter().enumerate() {
            let (mut signature, mut pub_key) = (
                util::encode_hex(input.signature.clone()),
                util::encode_hex(input.pub_key.clone()),
            );
            if compress {
                if input.signature.len() > 0 && compress {
                    signature = signature[..32].to_owned() + "...";
                }
                pub_key = pub_key[..32].to_owned() + "...";
            }
            let mut input_record = vec![
                Cell::new(&format!("{}", idx)),
                Cell::new(&util::encode_hex(&input.txid)),
                Cell::new(&format!("{}", input.vout)),
                Cell::new(&signature),
                Cell::new(&pub_key),
            ];
            input_records.push(Row::new(input_record));
        }
        for (idx, output) in self.vout.iter().enumerate() {
            let output_record = vec![
                Cell::new(&format!("{}", idx)),
                Cell::new(&format!("{:?}", output.value)),
                Cell::new(&util::encode_hex(&output.pub_key_hash)),
            ];
            output_records.push(Row::new(output_record));
        }
        (txid, input_records, output_records)
    }

    // TrimmedCopy creates a trimmed copy of Transaction to be used in signing
    // not include signature and pub_key.
    pub fn trimmed_copy(&self) -> Self {
        let mut inputs: Vec<TXInput> = vec![];
        let mut outputs: Vec<TXOutput> = vec![];

        self.vin.iter().for_each(|vin| {
            let tx = TXInput::new(vin.txid.clone(), vin.vout.clone(), vec![], vec![]);
            inputs.push(tx);
        });
        self.vout.iter().for_each(|vout| outputs.push(vout.clone()));

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
    pub fn verify(&self, prev_txs: &HashMap<isize, Transaction>) -> bool {
        if self.is_coinbase() {
            return true;
        }

        // check input wether reference some pre block output
        for (idx, _) in self.vin.iter().enumerate(){
            assert!(prev_txs.get(&(idx as isize)).is_some());
        }

        let tx_copy = &mut self.trimmed_copy();
        let mut verify = true;
        for (idx, tx_input) in self.vin.iter().enumerate() {
            if !verify {
               break; 
            }
            let prev_tx: &Transaction = prev_txs.get(&(idx as isize)).unwrap();
            tx_copy.vin[idx as usize].signature = vec![];
            tx_copy.vin[idx as usize].pub_key =
                prev_tx.vout[tx_input.vout as usize].pub_key_hash.clone();

            let origin_data_to_sign = util::packet_sign_content(&tx_copy);
            verify = util::verify(&tx_input.pub_key, &tx_input.signature, origin_data_to_sign);
            tx_copy.vin[idx as usize].pub_key = vec![];
        }
        verify
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
    pub fn uses_key(&self, pub_key: &[u8]) -> bool {
        util::compare_slice_u8(&self.pub_key, pub_key)
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
    pub fn deserialize_outputs(data: &[u8]) -> TXOutputs {
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
        }
    }


    #[test]
    fn txoutputs() {}
}
