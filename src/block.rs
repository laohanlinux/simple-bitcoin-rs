extern crate sha2;

use sha2::{Sha256, Digest};

use std::io::Write;

struct Block {
    timestamp: i64,
    data: Vec<u8>,
    prev_block_hash: Vec<u8>,
    hash: Vec<u8>,
}

impl Block {
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
