extern crate secp256k1;
extern crate rand;
extern crate sha2;
extern crate serde;
extern crate serde_json;
extern crate hex;

use self::secp256k1::{Message, ContextFlag};
use self::secp256k1::key::{SecretKey, PublicKey};
use self::rand::{Rng, thread_rng};
use self::sha2::{Sha256, Digest};

use super::util;
use std::sync::{Arc, Mutex};

const VERSION: u8 = 0u8;
const ADDRESS_CHECKSUM_LEN: usize = 4;

#[derive(Serialize, Deserialize, Debug)]
pub struct Wallet {
    secret_key: SecretKey,
    public_key: PublicKey,
}

impl Wallet {
    pub fn new() -> Wallet {
        let full = secp256k1::Secp256k1::with_caps(ContextFlag::Full);
        let (secret_key, public_key) = full.generate_keypair(&mut thread_rng()).unwrap();
        Wallet {
            secret_key: secret_key,
            public_key: public_key,
        }
    }

    pub fn new_key_pair() -> (SecretKey, Vec<u8>) {
        let (secret_key, public_key) = util::new_key_pair();
        (secret_key, util::public_key_to_vec(&public_key, false))
    }

    // get bitcoin address
    pub fn get_addrees(&self) -> String {
        // rimpemd160 20bytes
        let mut public_key = Self::hash_pubkey(&util::public_key_to_vec(&self.public_key, false));
        let mut version_payload = vec![VERSION];
        // 0x1|rimpemd160
        util::vec_stack_push(&mut public_key, 1);
        let mut address_sum = util::checksum_address(&public_key).split_off(ADDRESS_CHECKSUM_LEN);

        // packet base58 payload
        let mut full_payload = Vec::new();
        {
            full_payload.append(&mut version_payload);
            full_payload.append(&mut public_key);
            full_payload.append(&mut address_sum);
        }

        // base58
        util::encode_base58(&full_payload)
    }

    pub fn validate_address(address: String) -> bool {
        let base58_decode: Vec<u8> = util::decode_base58(address);
        let public_key = base58_decode.as_slice();
        let target_checksum = {
            let (start, end) = (0, public_key.len() - ADDRESS_CHECKSUM_LEN);
            let checked_text = &public_key[start..end];
            util::checksum_address(checked_text)
        };
        let actual_checksum = &public_key[(public_key.len() - ADDRESS_CHECKSUM_LEN)..];
        // 1. check address sum
        if util::compare_slice_u8(&target_checksum, &actual_checksum) == false {
            return false;
        }

        let version_slice = &public_key[..1];
        // 2. check version
        if util::compare_slice_u8(version_slice, &vec![VERSION]) == false {
            return false;
        }

        true
    }

    // 1. sha256  2. ripmed160
    fn hash_pubkey(public_key: &[u8]) -> Vec<u8> {
        let public_sha256 = util::sha256(public_key);
        util::encode_ripemd160(&public_sha256)
    }
}
