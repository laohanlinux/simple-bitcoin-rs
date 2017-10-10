use std::cell::RefCell;

use super::blockchain::BlockChain;

struct UTXOSet {
    blockchain: RefCell<BlockChain>,
}
