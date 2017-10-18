extern crate hex;
extern crate secp256k1;
extern crate sha2;
extern crate crypto;
extern crate crc;
extern crate rust_base58;
extern crate compare;
extern crate rand;
extern crate quick_error;
extern crate byteorder;
extern crate bigint;

use super::transaction;
use super::error::Error;

use self::bigint::U256;
use self::bigint::uint::*;

use self::sha2::{Sha256, Digest as Sha256Digest};
use self::secp256k1::{Signature, Secp256k1, Message, ContextFlag};
use self::secp256k1::key::{SecretKey, PublicKey};
use self::crypto::ripemd160;
use self::crypto::digest::Digest as Ripemd160Digest;
use self::crc::crc32;
use self::rust_base58::{ToBase58, FromBase58};
use self::compare::Compare;
use self::rand::thread_rng;
use self::quick_error::ResultExt;
use self::byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};

use std::fs::{File, OpenOptions};
use std::io::Cursor;
use std::path::Path;
use std::io::BufReader;
use std::io::prelude::*;
use std::cmp::Ordering::Equal;

pub fn write_i64(num: i64) -> Vec<u8> {
    let mut buf = Vec::with_capacity(0);
    buf.write_i64::<BigEndian>(num).unwrap();
    assert_eq!(buf.len(), 8);
    buf
}

pub fn write_i32(num: i32) -> Vec<u8> {
    let mut buf = Vec::with_capacity(0);
    buf.write_i32::<BigEndian>(num).unwrap();
    assert_eq!(buf.len(), 4);
    buf
}

pub fn write_u32(num: u32) -> Vec<u8> {
    let mut buf = Vec::with_capacity(0);
    buf.write_u32::<BigEndian>(num).unwrap();
    assert_eq!(buf.len(), 4);
    buf
}

pub fn write_u8(num: u8) -> Vec<u8> {
    let mut buf = Vec::with_capacity(0);
    buf.write_u8(num).unwrap();
    assert_eq!(buf.len(), 1);
    buf
}

pub fn read_i64(buf: &[u8]) -> i64 {
    assert_eq!(buf.len(), 8);
    let mut rdr = Cursor::new(buf);
    rdr.read_i64::<BigEndian>().unwrap()
}

pub fn read_i32(buf: &[u8]) -> i32 {
    assert_eq!(buf.len(), 4);
    let mut rdr = Cursor::new(buf);
    rdr.read_i32::<BigEndian>().unwrap()
}

pub fn read_u32(buf: &[u8]) -> u32 {
    assert_eq!(buf.len(), 4);
    let mut rdr = Cursor::new(buf);
    rdr.read_u32::<BigEndian>().unwrap()
}

pub fn read_u8(buf: &[u8]) -> u8 {
    assert_eq!(buf.len(), 1);
    let mut rdr = Cursor::new(buf);
    rdr.read_u8().unwrap()
}

pub fn read_file(path: &str) -> Result<Vec<u8>, Error> {
    let path = Path::new(path);
    let file = File::open(path).context(path)?;
    let mut buf_reader = BufReader::new(file);
    let mut buf = vec![];
    buf_reader.read_to_end(&mut buf).context(path)?;
    Ok(buf)
}

pub fn write_file(path: &str, contents: &[u8]) -> Result<(), Error> {
    let path = Path::new(path);
    let mut f = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .context(path)?;
    f.write_all(contents).context(path)?;
    f.sync_all().context(path)?;
    Ok(())
}

pub fn compare_slice_u8(s1: &[u8], s2: &[u8]) -> bool {
    println!("compare: {:?}, {:?}", s1, s2);
    let cmp = |l: &[u8], r: &[u8]| l.len().cmp(&r.len());
    cmp.compare(s1, s2) == Equal
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

pub fn as_u256(data: &[u8]) -> U256 {
    U256::from_big_endian(data)
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
    write_u32(crc32(&sha256(&next)))
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
    let s = Secp256k1::with_caps(ContextFlag::Full);
    SecretKey::from_slice(&s, origin_secret_key).unwrap()
}

pub fn new_key_pair() -> (SecretKey, PublicKey) {
    let full = Secp256k1::new();
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

#[cfg(test)]
mod tests {
    #[test]
    fn test_bin_op() {
        use super::{write_i32, write_i64, read_i32, read_i64};

        let test_i32 = 1 << 30;
        let write_i32_res = write_i32(test_i32);
        assert_eq!(read_i32(&write_i32_res), test_i32);

        let test_i64 = 1 << 63;
        let write_i64_res = write_i64(test_i64);
        assert_eq!(read_i64(&write_i64_res), test_i64);
    }

    #[test]
    fn test_compare() {
        let s1 = [
            1,
            206,
            137,
            99,
            239,
            57,
            212,
            134,
            92,
            105,
            90,
            64,
            29,
            64,
            152,
            90,
            38,
            111,
            49,
            250,
            0,
            60,
            219,
            82,
            167,
            191,
            179,
            42,
            173,
            234,
            43,
            86,
            95,
            115,
            98,
            108,
            251,
            120,
            86,
            167,
            75,
            128,
            253,
            228,
            239,
        ];
        let s2 = [
            1,
            206,
            137,
            99,
            239,
            57,
            212,
            134,
            92,
            105,
            90,
            64,
            29,
            64,
            152,
            90,
            38,
            111,
            49,
            250,
            0,
            60,
            219,
            82,
            167,
            191,
            179,
            42,
            173,
            234,
            43,
            86,
            95,
            115,
            98,
            108,
            251,
            120,
            86,
            167,
            75,
            128,
            253,
            228,
            239,
        ];
        assert_eq!(s1.len(), s2.len());

        assert!(super::compare_slice_u8(&s1, &s2));

        let s2 = [31, 254, 100, 67, 198, 7, 126, 136, 53, 144, 173, 193, 233, 118, 113, 198, 218, 220, 70, 61];
        let s1 = [75, 137, 91, 42, 254, 205, 222, 47, 210, 83, 130, 161, 129, 218, 136, 215, 116, 211, 90, 251]; 
        assert!(!super::compare_slice_u8(&s1, &s2));
    }
}
