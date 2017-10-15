extern crate secp256k1;
extern crate rand;
extern crate sha2;
extern crate serde;
extern crate serde_json;
extern crate hex;
extern crate slog;
extern crate slog_term;

use self::secp256k1::{ContextFlag};
use self::secp256k1::key::{SecretKey, PublicKey};
use self::rand::{thread_rng};

use super::log::*;
use super::util;
use std::sync::{Arc, Mutex};

const NETENV: u8 = 0u8;

pub const ADDRESS_CHECKSUM_LEN: usize = 4;

#[derive(Serialize, Deserialize, Debug)]
pub struct Wallet {
    pub secret_key: SecretKey,
    pub public_key: PublicKey,
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
        let mut version_payload = util::write_u8(NETENV);
        // 0x00x1|rimpemd160
        
        let mut version_payload_clone = version_payload.clone();
        {
            version_payload_clone.append(&mut public_key);
            public_key = version_payload_clone;
        }
        
        assert!(public_key.len() == 21);
        let mut address_sum = util::checksum_address(&public_key);
        assert!(address_sum.len() == 4);
        // packet base58 payload
        let mut full_payload = Vec::new();
        {
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
            warn!(LOG, "address checksum is not equal");
            return false;
        }
        let netenv = util::read_u8(&public_key[..1]);
        if netenv != NETENV {
            warn!(
                LOG,
                "address version is valid, {:?}, {:?}",
                netenv,
                NETENV
            );
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


#[cfg(test)]
mod tests {
    use super::util;
    use super::Wallet;

    #[test]
    fn test_wallet() {
        let new_wallet = Wallet::new();
        println!("{}", util::public_key_to_vec(&new_wallet.public_key, true).len());
        assert!(util::public_key_to_vec(&new_wallet.public_key, false).len() == 65);
        assert!(util::public_key_to_vec(&new_wallet.public_key, true).len() == 33);

        let addr = new_wallet.get_addrees();
        println!("addr {}", addr);
        assert!(Wallet::validate_address(addr));
    }
}