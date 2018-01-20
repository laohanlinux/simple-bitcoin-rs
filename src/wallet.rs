extern crate secp256k1;
extern crate rand;
extern crate sha2;
extern crate serde;
extern crate serde_json;
extern crate hex;
extern crate slog;
extern crate slog_term;

use self::secp256k1::ContextFlag;
use self::secp256k1::key::{SecretKey, PublicKey};
use self::rand::thread_rng;

use super::log::*;
use super::util;

use std::collections::HashMap;

const NET_ENV: u8 = 0u8;

pub const ADDRESS_CHECKSUM_LEN: usize = 4;

#[derive(Serialize, Deserialize, Debug)]
pub struct Wallet {
    pub secret_key: SecretKey,
    #[serde(default)]
    pub secret_key_hex: String,
    pub public_key: PublicKey,
}

impl Wallet {
    pub fn new() -> Wallet {
        let full = secp256k1::Secp256k1::with_caps(ContextFlag::Full);
        let (secret_key, public_key) = full.generate_keypair(&mut thread_rng()).unwrap();
        let mut w = Wallet {
            secret_key: secret_key,
            secret_key_hex: "..".to_owned(),
            public_key: public_key,
        };
        let (secret_key_vec, _) = w.to_vec();
        w.secret_key_hex = util::encode_hex(&secret_key_vec);
        w
    }

    pub fn new_key_pair() -> (SecretKey, Vec<u8>) {
        let (secret_key, public_key) = util::new_key_pair();
        (secret_key, util::public_key_to_vec(&public_key, false))
    }

    pub fn recover_wallet(origin_secret_key: &[u8]) -> Result<Wallet, String> {
        let secret_key = util::try_recover_secret_key(origin_secret_key).map_err(
            |e| {
                format!("{:?}", e)
            },
        )?;
        let secp = secp256k1::Secp256k1::with_caps(ContextFlag::Full);
        let pub_key = PublicKey::from_secret_key(&secp, &secret_key).unwrap();
        let mut w = Wallet {
            secret_key: secret_key,
            secret_key_hex: "..".to_owned(),
            public_key: pub_key,
        };
        let (secret_key_vec, _) = w.to_vec();
        w.secret_key_hex = util::encode_hex(&secret_key_vec);
        Ok(w)
    }

    // get bitcoin address
    pub fn get_address(&self) -> String {
        // rimpemd160 20bytes
        let mut public_key = Self::hash_pubkey(&util::public_key_to_vec(&self.public_key, false));
        let version_payload = util::write_u8(NET_ENV);
        // 0x00x1|rimpemd160
        let mut version_payload_clone = version_payload.clone();
        {
            version_payload_clone.append(&mut public_key);
            public_key = version_payload_clone;
        }

        assert_eq!(public_key.len(), 21);
        let mut address_sum = util::checksum_address(&public_key);
        assert_eq!(address_sum.len(), 4);
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
        let net_env = util::read_u8(&public_key[..1]);
        if net_env != NET_ENV {
            warn!(LOG, "address version is valid, {:?}, {:?}", net_env, NET_ENV);
            return false;
        }
        true
    }

    // 1. sha256  2. ripmed160
    pub fn hash_pubkey(public_key: &[u8]) -> Vec<u8> {
        let public_sha256 = util::sha256(public_key);
        util::encode_ripemd160(&public_sha256)
    }

    pub fn serialize(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap()
    }

    pub fn to_vec(&self) -> (Vec<u8>, Vec<u8>) {
        let tmp_pair = PrivatePair {
            public_key: self.public_key.clone(),
            secret_key: self.secret_key.clone(),
        };
        let serialize_vec = serde_json::to_vec(&tmp_pair).unwrap();
        let pair: HashMap<String, Vec<u8>> = serde_json::from_slice(&serialize_vec).unwrap();
        let secret_key = pair.get("secret_key").unwrap();
        let public_key = pair.get("public_key").unwrap();
        (secret_key.clone(), public_key.clone())
    }

    pub fn to_btc_pair(&self) -> BTCPair {
        let (secret_key, public_key) = self.to_vec();
        let secret_key_hex = util::encode_hex(&secret_key);
        let address = self.get_address();
        BTCPair {
            secret_key: secret_key,
            secret_key_hex: secret_key_hex,
            public_key: public_key,
            address: address,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct PrivatePair {
    secret_key: SecretKey,
    public_key: PublicKey,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BTCPair {
    secret_key: Vec<u8>,
    secret_key_hex: String,
    public_key: Vec<u8>,
    address: String,
}

//#[cfg(test)]
/*mod tests {
    use super::util;
    use super::Wallet;

    #[test]
    fn test_wallet() {
        let new_wallet = Wallet::new();
        println!(
            "{}",
            util::public_key_to_vec(&new_wallet.public_key, true).len()
        );
        assert!(util::public_key_to_vec(&new_wallet.public_key, false).len() == 65);
        assert!(util::public_key_to_vec(&new_wallet.public_key, true).len() == 33);

        let addr = new_wallet.get_addrees();
        println!("addr {}", addr);
        assert!(Wallet::validate_address(addr));
    }
}*/
