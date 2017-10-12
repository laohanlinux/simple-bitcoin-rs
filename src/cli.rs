extern crate slog;
extern crate slog_term;

use super::util;
use super::log::*;
use super::wallets::Wallets;
use slog::*;

use std::fs;

pub fn create_wallet(node: String, del_old: bool) {
    let wallets = Wallets::new().unwrap();
    wallets.save_to_file(&node);
    info!(
        LOG,
        "All your wallet  address:",
    );
    let address = wallets.list_address();
    address.into_iter().fold(0, |acc, addr| {
        info!(LOG, "地址[{:?}]=> {:?}", acc, addr);
        acc + 1
    });
    if del_old {
        fs::remove_file(&node).unwrap();
    }
}

pub fn open_wallet(node: String) {}
