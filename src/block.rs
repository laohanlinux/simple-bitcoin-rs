extern crate serde;
extern crate serde_json;
extern crate hex;

extern crate sha2;
extern crate time;

use self::sha2::{Sha256, Digest};
use super::util;
use super::proof_of_work;
use super::transaction::*;
use super::merkle_tree::MerkleTree;

#[derive(Serialize, Deserialize, Debug)]
pub struct Block {
    timestamp: i64,
    prev_block_hash: Vec<u8>,
    hash: Vec<u8>,

    transactions: Vec<Transaction>,
    nonce: isize,
    height: isize,
}

impl Block {
    pub fn new(transactions: Vec<Transaction>, prev_block_hash: Vec<u8>, height: isize) -> Block {
        let mut block = Block {
            timestamp: time::get_time().sec,
            prev_block_hash: prev_block_hash,
            hash: vec![],
            transactions: transactions,
            nonce: 0,
            height: height,
        };
        let mut pow = proof_of_work::ProofOfWork::new_proof_of_work(&block);

        let (nonce, hash) = pow.run();
        pow.block.hash = hash;
        pow.block.nonce = nonce;
        block
    }

    pub fn serialize(block: &Block) -> Vec<u8> {
        serde_json::to_string(block).unwrap().into_bytes()
    }

    pub fn deserialize_block(data: &Vec<u8>) -> Self {
        serde_json::from_str(&String::from_utf8(data.clone()).unwrap()).unwrap()
    }

    fn new_genesis_block(coinbase: Transaction) -> Self {
        let mut block: Block = Default::default();
        block.transactions = coinbase;
        block
    }

    // hash(prev_block_hash|data|timestamp)
    fn set_hash(&mut self) {
        let mut timestamp_buf = Vec::new();
        write!(&mut timestamp_buf, "{}", self.timestamp).unwrap();

        let mut header = Vec::new();
        header.append(&mut self.prev_block_hash.clone());
        header.append(&mut self.data.clone());
        header.append(&mut timestamp_buf);

        let mut hasher = Sha256::default();
        hasher.input(&header);

        self.hash = hasher.result().to_vec();
    }

    pub fn hash_transactions(&self) -> Vec<u8> {
        let mut transaction = vec![];
        for tx in &self.transactions {
            transaction.push(tx.serialize());
        }

        let merkle_tree: MerkleTree = MerkleTree::new_merkle_tree(transaction);
        *merkle_tree.root.unwrap()
    }
}

//#[cfg(test)]
//mod tests {
//    extern crate hex;
//
//    use std::io::{self, Write};
//
//    #[test]
//    fn it_works() {}
//
//    #[test]
//    fn blockchain() {
//        let mut block_chain = ::block::BlockChain::new();
//        block_chain.add("Send 1 BTC to Ivan".to_string());
//        block_chain.add("Send 2 more BTC to Ivan".to_string());
//        let blocks = block_chain.blocks.get_mut();
//        for block in blocks {
//            let timestamp = &block.timestamp;
//            let hash = &block.hash;
//            let data = &block.data;
//            let prev_block_hash = &block.prev_block_hash;
//            writeln!(
//                io::stdout(),
//                "timestamp: {:?}, hash:{}, prev_hash:{:?}, data: {:?}",
//                timestamp,
//                hex::encode(hash),
//                hex::encode(prev_block_hash),
//                data
//            );
//        }
//    }
//}
