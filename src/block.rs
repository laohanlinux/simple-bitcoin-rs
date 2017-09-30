extern crate serde;
extern crate serde_json;

extern crate sha2;
extern crate time;

use std::io::Write;
use std::cell::RefCell;

use self::sha2::{Sha256, Digest};

const DBFILE: &str = "blockchain.db";
const BLOCK_PREFIX: &str = "blocks";
const GENESIS_COINBASE_DATA: &str = "The Times 03/Jan/2009 Chancellor on brink of second bailout for banks";

#[derive(Serialize, Deserialize, Debug)]
pub struct Block {
    timestamp: i64,
    data: Vec<u8>,
    prev_block_hash: Vec<u8>,
    hash: Vec<u8>,
}

impl Block {
    pub fn new(data: String, prev_block_hash: Vec<u8>) -> Block {
        let data_bytes = data.into_bytes();
        let mut block = Block {
            timestamp: time::get_time().sec,
            data: data_bytes,
            prev_block_hash: prev_block_hash,
            hash: Vec::new(),
        };
        block.set_hash();
        block
    }

    pub fn serialize(block: &Block) -> Vec<u8> {
        serde_json::to_string(block).unwrap().into_bytes()
    }

    pub fn deserialize_block(data: &Vec<u8>) -> Self {
        serde_json::from_str(&String::from_utf8_lossy(data)).unwrap()
    }

    fn new_genesis_block() -> Self {
        ::block::Block::new("Genesis Block".to_string(), Vec::new())
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
}

#[derive(Debug)]
pub struct BlockChain {
    blocks: RefCell<Vec<Block>>,
}

impl BlockChain {
    fn new() -> Self {
        let mut blocks = Vec::new();
        blocks.push(::block::Block::new_genesis_block());
        BlockChain { blocks: RefCell::new(blocks) }
    }

    fn add(&self, data: String) {
        let mut new_block = Block::new_genesis_block();
        {
            let bs = self.blocks.borrow();
            let prev_block = bs.last().unwrap();
            let prev_hash = prev_block.hash.clone();
            new_block = Block::new(data, prev_hash);
        }
        {
            let mut bs = self.blocks.borrow_mut();
            bs.push(new_block);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{self, Write};
    #[test]
    fn it_works() {}

    #[test]
    fn blockchain() {
        let mut block_chain = ::block::BlockChain::new();
        block_chain.add("Send 1 BTC to Ivan".to_string());
        block_chain.add("Send 2 more BTC to Ivan".to_string());
        let blocks = block_chain.blocks.get_mut();
        for block in blocks {
            let timestamp = &block.timestamp;
            let hash = &block.hash;
            let data = &block.data;
            let prev_block_hash = &block.prev_block_hash;
            writeln!(
                io::stdout(),
                "timestamp: {:?}, hash:{}, prev_hash:{:?}, data: {:?}",
                timestamp,
                String::from_utf8_lossy(hash).to_string(),
                prev_block_hash,
                data
            );
        }
    }
}
