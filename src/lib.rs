#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]
#![feature(attr_literals)]
#![allow(unused_variables)]

#[macro_use] extern crate rocket_contrib;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate quick_error;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate runtime_fmt;

#[macro_use]
extern crate slog;
extern crate slog_term;

#[macro_use]
extern crate bigint;

mod error;
mod block;
mod blockchain;
mod utxo_set;
mod wallet;
mod wallets;
mod db;
mod util;
mod merkle_tree;
mod transaction;
mod proof_of_work;
mod http_server;
mod cli;
mod log;
