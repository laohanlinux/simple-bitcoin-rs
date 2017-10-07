extern crate hex;
extern crate secp256k1;
extern crate sha2;

use self::sha2::{Sha256, Digest};
use super::transaction;
use self::secp256k1::{Signature, Secp256k1, Message, ContextFlag};
use self::secp256k1::key::{SecretKey, PublicKey};

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub fn encode_hex<T: AsRef<[u8]>>(data: T) -> String {
    hex::encode(data)
}

pub fn decode_hex<T: AsRef<[u8]>>(data: T) -> Vec<u8> {
    hex::decode(data).unwrap()
}

pub fn double_sha256(input_str: String) -> Vec<u8> {
    let next = sha256(input_str.as_bytes());
    sha256(&next)
}

pub fn packet_sign_content(tx: &transaction::Transaction) -> String {
    format!("{:?}", tx)
}

pub fn recover_secret_key(origin_secret_key: &[u8]) -> SecretKey {
    let s = Secp256k1::new();
    SecretKey::from_slice(&s, origin_secret_key).unwrap()
}

// return signature der string
pub fn sign(msg: &Message, secret_key: &SecretKey) -> Vec<u8> {
    let full = Secp256k1::with_caps(ContextFlag::Full);
    let sig = full.sign(msg, secret_key).unwrap();
    sig.serialize_der(&full)
}

pub fn verify(pub_key: &[u8], sig_str: &[u8], origin_data_to_sign: String) -> bool {
    let data_to_sign = double_sha256(origin_data_to_sign);
    let full = Secp256k1::with_caps(ContextFlag::Full);
    let recover_sig = Signature::from_der(&full, sig_str).unwrap();
    let recover_pub_key = PublicKey::from_slice(&full, pub_key).unwrap();
    full.verify(
        &Message::from_slice(&data_to_sign).unwrap(),
        &recover_sig,
        &recover_pub_key,
    ).is_ok()
}

fn sha256(input: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::default();
    hasher.input(input);
    hasher.result().to_vec()
}
