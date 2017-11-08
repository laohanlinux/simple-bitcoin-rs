#![feature(attr_literals)]
#![allow(unused_variables)]
#![feature(plugin)]
#![plugin(rocket_codegen)]

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
#[macro_use]
extern crate slog_term;

#[macro_use]
extern crate bigint;

#[macro_use]
extern crate rocket_contrib;

extern crate rocket;

#[macro_use]
extern crate validator_derive;

#[macro_use]
extern crate validator;


#[macro_use]
pub mod comm;

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
mod server;
mod command;
mod router;

pub mod cli;
pub mod log;

