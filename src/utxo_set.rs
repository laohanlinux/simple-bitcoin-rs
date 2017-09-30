use std::cell::RefCell;
use block;

struct UTXOSet {
    blockchain: RefCell<block::BlockChain>,
}
