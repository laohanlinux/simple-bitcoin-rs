extern crate secp256k1;


use std::sync::{Arc, Mutex};

use self::secp256k1::*;

pub struct Wallet {
    public_key: Vec<u8>,
}
