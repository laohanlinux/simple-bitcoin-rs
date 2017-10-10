use std::cell::RefCell;

use super::blockchain::BlockChain;

pub struct UTXOSet {
    pub blockchain: RefCell<BlockChain>,
}
