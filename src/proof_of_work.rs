extern crate bigint;
extern crate lazy_static;

use self::bigint::U256;

use super::block::*;
use super::util;

// TODO
lazy_static!{
    static ref MAX_NONCE: U256 = 10.into();
}

const TARGETBITS: usize = 16;

pub struct ProofOfWork<'a> {
    pub block: &'a Block,
    target: U256,
}

impl<'a> ProofOfWork<'a> {
    const target_bits: i64 = 16;
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

    /**
    |prevblock_hash|block_transaction_hash|timestamp|targetbits|nonce|
    */
    pub fn prepare_data(&self, nonce: isize) -> Vec<u8> {
        let prev_block_hash = &self.block.prev_block_hash;
        let prev_block_end = prev_block_hash.len();
        let hash_transactions = &self.block.hash_transactions();
        let hash_transactions_end = prev_block_end + hash_transactions.len();
        let timestamp = &util::write_i64(self.block.timestamp);
        let timestamp_end = hash_transactions_end + timestamp.len();
        let target_bits = &util::write_i64(Self::target_bits);
        let target_bits_end = timestamp_end + target_bits.len();
        let nonce = &util::write_i64(nonce as i64);
        let nonce_end = target_bits_end + nonce.len();

        let buf_size = timestamp.len()
            + nonce.len()
            + target_bits.len()
            + self.block.prev_block_hash.len()
            + hash_transactions.len();

        let mut buf = Vec::with_capacity(buf_size);
        buf[..prev_block_end].clone_from_slice(prev_block_hash);
        buf[prev_block_end..hash_transactions_end].clone_from_slice(hash_transactions);
        buf[hash_transactions_end..timestamp_end].clone_from_slice(hash_transactions);
        buf[timestamp_end..target_bits_end].clone_from_slice(target_bits);
        buf[target_bits_end..nonce_end].clone_from_slice(nonce);

        buf
    }

    pub fn validate(&self) -> bool {
        let hash_data = util::sha256(&self.prepare_data(self.block.nonce));
        let hash_dec = util::encode_hex(hash_data);
        let hash_big = U256::from_dec_str(&hash_dec).expect("hash is too bigger");
        hash_big < self.target
    }
}
