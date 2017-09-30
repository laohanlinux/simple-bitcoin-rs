extern crate fastcmp;

use fastcmp::Compare;

pub struct TXTInput {
    txid: Vec<u8>,
    vout: i32,
    signature: Vec<u8>,
    pub_key: Vec<u8>,
}

impl TXTInput {
    pub fn uses_key(&self, pub_key: &Vec<u8>) -> bool {
        &self.pub_key.feq(pub_key)
    }
}
