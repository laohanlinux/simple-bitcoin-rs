extern crate bigint;
extern crate lazy_static;
extern crate slog;
extern crate slog_term;

use self::bigint::U256;

use super::block::*;
use super::util;
use super::log::*;

use std::iter;

// TODO
lazy_static!{
    static ref MAX_NONCE: isize =  1<<60;
}

//const TARGETBITS: usize = 16;
const TARGETBITS: usize = 16;

pub struct ProofOfWork<'a> {
    pub block: &'a Block,
    target: U256,
}

impl<'a> ProofOfWork<'a> {
    const TARGET_BITS: i64 = 16;
    pub fn new_proof_of_work(b: &'a Block) -> ProofOfWork<'a> {
        let target: U256 = 1.into();
        // left move 240 bits
        let target: U256 = target << (256 - TARGETBITS);

        ProofOfWork {
            block: b,
            target: target,
        }
    }

    // TODO add debug infomation
    pub fn run(&self) -> (isize, Vec<u8>) {
        let mut nonce = 0;
        let mut hash = vec![];
        
        for n in 0..*MAX_NONCE {
            let data = self.prepare_data(n);
            nonce = n;
            hash = util::sha256(&data);
            let hash_int: U256 = util::as_u256(&hash);
            if hash_int < self.target {
                break;
            }
        }
        assert!(nonce != *MAX_NONCE);
        (nonce, hash)
    }

    pub fn validate(&self) -> bool {
        let hash_data = util::sha256(&self.prepare_data(self.block.nonce));
        let hash_big = util::as_u256(&hash_data);
        hash_big < self.target
    }

    // |prevblock_hash|block_transaction_hash|timestamp|targetbits|nonce|
    fn prepare_data(&self, nonce: isize) -> Vec<u8> {
        let prev_block_hash = &self.block.prev_block_hash;
        let prev_block_end = prev_block_hash.len();
        let hash_transactions = &self.block.hash_transactions();
        let hash_transactions_end = prev_block_end + hash_transactions.len();
        let timestamp = &util::write_i32(self.block.timestamp);
        let timestamp_end = hash_transactions_end + timestamp.len();
        let target_bits = &util::write_i64(Self::TARGET_BITS);
        let target_bits_end = timestamp_end + target_bits.len();
        let nonce = &util::write_i64(nonce as i64);
        let nonce_end = target_bits_end + nonce.len();

        let buf_size = timestamp.len() + nonce.len() + target_bits.len() +
            self.block.prev_block_hash.len() + hash_transactions.len();

        let mut buf = Vec::with_capacity(buf_size);
        buf.extend(iter::repeat(0).take(buf_size));
        buf[..prev_block_end].clone_from_slice(prev_block_hash);
        buf[prev_block_end..hash_transactions_end].clone_from_slice(hash_transactions);
        buf[hash_transactions_end..timestamp_end].clone_from_slice(timestamp);
        buf[timestamp_end..target_bits_end].clone_from_slice(target_bits);
        buf[target_bits_end..nonce_end].clone_from_slice(nonce);

        buf
    }
}
