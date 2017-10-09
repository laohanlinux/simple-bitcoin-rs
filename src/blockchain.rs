use std::io::Write;
use std::cell::RefCell;

use super::block::*;

const DBFILE: &str = "blockchain.db";
const BLOCK_PREFIX: &str = "blocks";
const GENESIS_COINBASE_DATA: &str = "The Times 03/Jan/2009 Chancellor on brink of second bailout for banks";

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