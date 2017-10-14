extern crate serde;
extern crate serde_json;
extern crate hex;

extern crate sha2;
extern crate time;

use super::proof_of_work;
use super::transaction::*;
use super::merkle_tree::MerkleTree;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Block {
    pub timestamp: i64,
    pub prev_block_hash: Vec<u8>,
    pub transactions: Vec<Transaction>,
    pub nonce: isize,

    // hash = hash_fn(timestamp|prev_block|transactions|nonce), no include height
    pub hash: Vec<u8>,
    pub height: isize,
}

impl Block {
    pub fn new(transactions: Vec<Transaction>, prev_block_hash: Vec<u8>, height: isize) -> Block {
        let block = Block {
            timestamp: time::get_time().sec,
            prev_block_hash: prev_block_hash,
            transactions: transactions,
            height: height,
            ..Default::default()
        };
        let pow = proof_of_work::ProofOfWork::new_proof_of_work(&block);

        let (nonce, hash) = pow.run();
        return {
            let mut block = pow.block.clone();
            block.hash = hash;
            block.nonce = nonce;
            block
        };
    }

    pub fn serialize(block: &Block) -> Vec<u8> {
        serde_json::to_string(block).unwrap().into_bytes()
    }

    pub fn deserialize_block(data: &Vec<u8>) -> Self {
        serde_json::from_str(&String::from_utf8(data.clone()).unwrap()).unwrap()
    }

    pub fn new_genesis_block(coinbase: Transaction) -> Self {
        let block: Block = Block::new(vec![coinbase], vec![], 0);
        block
    }

    pub fn hash_transactions(&self) -> Vec<u8> {
        let mut transaction = vec![];
        for tx in &self.transactions {
            transaction.push(tx.serialize());
        }

        let merkle_tree: MerkleTree = MerkleTree::new_merkle_tree(transaction);
        *merkle_tree.root.unwrap().data
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
