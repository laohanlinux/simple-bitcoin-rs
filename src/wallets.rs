extern crate serde;
extern crate serde_json;
extern crate hex;

use super::wallet::Wallet;
use super::util;
use super::error::Error;

use std::collections::HashMap;

const WALLET_FILE: &str = "Wallet_%s.data";

#[derive(Serialize, Deserialize, Debug)]
struct Wallets {
    wallets: HashMap<String, Wallet>,
}

impl Wallets {
    pub fn new_wallets(node: String) -> Result<Wallets, Error> {
        Ok(Self::load_from_file(&node))
    }

    // return new wallet address
    pub fn create_wallet(&mut self) -> String {
        let wallet = Wallet::new();
        let address = wallet.get_addrees();
        self.wallets.insert(address.clone(), wallet);
        address
    }

    pub fn get_wallet(&self, address: String) -> Option<&Wallet> {
        self.wallets.get(&address)
    }

    fn load_from_file(node: &str) -> Wallets {
        let contents = util::read_file(node).unwrap();
        Self::deserialize(&contents)
    }

    fn save_to_file(&self, node: &str) {
        let contents = Self::serialize(self);
        util::write_file(node, &contents).unwrap();
    }

    fn serialize(wallets: &Wallets) -> Vec<u8> {
        serde_json::to_string(wallets).unwrap().into_bytes()
    }

    fn deserialize(data: &Vec<u8>) -> Wallets {
        serde_json::from_str(&String::from_utf8(data.clone()).unwrap()).unwrap()
    }
}
