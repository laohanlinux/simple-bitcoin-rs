extern crate sha2;
extern crate time;

use std::io::Write;

use sha2::{Sha256, Digest};

#[derive(Debug)]
struct Block {
    timestamp: i64,
    data: Vec<u8>,
    prev_block_hash: Vec<u8>,
    hash: Vec<u8>,
}

impl Block {
    fn new(data: String, prev_block_hash: Vec<u8>) -> Self {
        let mut block = Block {
            timestamp: time::get_time().sec,
            data: data.into_bytes(),
            prev_block_hash: prev_block_hash,
            hash: Vec::new()
        };
        block.set_hash();
        block
    }

    fn new_genesis_block() -> Self {
        ::block::Block::new("Genesis Block".to_string(), Vec::new())
    }

    // hash(prev_block_hash|data|timestamp)
    fn set_hash(&mut self) {
        let mut timestamp_buf = Vec::new();
        write!(&mut timestamp_buf, "{}", self.timestamp).unwrap();

        let mut header = Vec::new();
        header.append(&mut self.prev_block_hash);
        header.append(&mut self.data);
        header.append(&mut timestamp_buf);

        let mut hasher = Sha256::default();
        hasher.input(&header);

        self.hash = hasher.result().to_vec();
    }
}

#[derive(Debug)]
struct BlockChain {
    blocks: Vec<Block>,
}

impl BlockChain {
    fn new() -> Self {
        let mut blocks = Vec::new();
        blocks.push(::block::Block::new_genesis_block());
        BlockChain {
            blocks: blocks,
        }
    }

    fn add(&mut self, data: String) {
        let bs = self.blocks.as_mut();
//        if let Some(prev_block) = bs.last() {
//            bs.push(::block::Block::new_genesis_block());
//        }
//        {
//            let prev_block: &Block = bs.last().unwrap();
//            let prev_hash = prev_block.hash.clone();
//            let new_block = Block::new(data, prev_hash);
//        }
        //bs.push(new_block);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}

    #[test]
    fn blockchain() {
        let mut block_chain = ::block::BlockChain::new();
        block_chain.add("Send 1 BTC to Ivan".to_string());
        block_chain.add("Send 2 more BTC to Ivan".to_string());
        for block in &block_chain.blocks {
            println!("{:?}", block);
        }
    }
}
