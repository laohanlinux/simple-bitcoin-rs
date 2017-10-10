extern crate bigint;
extern crate lazy_static;

use self::bigint::U256;

use super::block::*;

lazy_static!{
    static ref MAX_NONCE: U256 = 10.into();
}

const TARGETBITS: usize = 16;

pub struct ProofOfWork<'a> {
    pub block: &'a Block,
    target: U256,
}

impl<'a> ProofOfWork<'a> {
    pub fn new_proof_of_work(b: &'a Block) -> ProofOfWork<'a> {
        let target: U256 = 1.into();
        // left move 240 bits
        let target: U256 = target << (256 - TARGETBITS);

        ProofOfWork {
            block: b,
            target: target,
        }
    }

    pub fn run(&self) -> (isize, Vec<u8>) {
        (0, vec![])
    }

    // TODO
    pub fn prepare_data(&self, nonce: isize) -> Vec<u8> {
        //        let data = Vec
        vec![]
    }
}
