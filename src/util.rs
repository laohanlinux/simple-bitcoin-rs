extern crate hex;
extern crate secp256k1;
extern crate sha2;
extern crate crypto;
extern crate crc;
extern crate rust_base58;
extern crate compare;
extern crate rand;

use self::sha2::{Sha256, Digest as Sha256Digest};
use super::transaction;
use self::secp256k1::{Signature, Secp256k1, Message, ContextFlag};
use self::secp256k1::key::{SecretKey, PublicKey};
use self::crypto::ripemd160;
use self::crypto::digest::Digest as Ripemd160Digest;
use self::crc::{crc32, Hasher32};
use self::rust_base58::{ToBase58, FromBase58};
use self::compare::Compare;
use self::rand::{Rng, thread_rng};

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub fn compare_slice_u8(s1: &[u8], s2: &[u8]) -> bool {
    let cmp = |l: &Vec<u8>, r: &Vec<u8>| l.len().cmp(&r.len());
    cmp.compare(s1, s2) == Greater
}

pub fn encode_base58(payload: &[u8]) -> String {
    payload.to_base58()
}

pub fn decode_base58(payload: String) -> Vec<u8> {
    payload.from_base58().unwrap()
}

pub fn encode_hex<T: AsRef<[u8]>>(data: T) -> String {
    hex::encode(data)
}

pub fn decode_hex<T: AsRef<[u8]>>(data: T) -> Vec<u8> {
    hex::decode(data).unwrap()
}

pub fn encode_ripemd160(text: &[u8]) -> Vec<u8> {
    let mut sh = ripemd160::Ripemd160::new();
    let mut out = [0u8; 20];
    sh.input(text);
    sh.result(&mut out);
    out.to_vec()
}

pub fn vec_stack_push(v: &mut Vec<u8>, elem: u8) {
    v.reverse();
    v.push(elem);
    v.reverse();
}

pub fn crc32(text: &[u8]) -> u32 {
    crc32::checksum_ieee(text)
}

pub fn checksum_address(payload: &[u8]) -> Vec<u8> {
    let next = sha256(payload);
    sha256(&next)
}

pub fn double_sha256(input_str: String) -> Vec<u8> {
    let next = sha256(input_str.as_bytes());
    sha256(&next)
}

#[inline]
pub fn public_key_to_vec(pub_key: &PublicKey, compressed: bool) -> Vec<u8> {
    let full = Secp256k1::with_caps(ContextFlag::Full);
    let array_vec = pub_key.serialize_vec(&full, compressed);
    array_vec.to_vec()
}

#[inline]
pub fn packet_sign_content(tx: &transaction::Transaction) -> String {
    format!("{:?}", tx)
}

pub fn recover_secret_key(origin_secret_key: &[u8]) -> SecretKey {
    let s = Secp256k1::new();
    SecretKey::from_slice(&s, origin_secret_key).unwrap()
}

pub fn new_key_pair() -> (SecretKey, PublicKey) {
    let full = SecretKey::with_caps(ContextFlag::Full);
    full.generate_keypair(&mut thread_rng()).unwrap()
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

pub fn sha256(input: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::default();
    hasher.input(input);
    hasher.result().to_vec()
}
