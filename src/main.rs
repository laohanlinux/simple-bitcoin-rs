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
extern crate clap;

use clap::{Arg, App, SubCommand, ArgMatches};

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

use log::*;

// takes_value() and default_value() to read values from arguments like --option=foo
fn main() {
    let matches = App::new("bitcoin")
        .version("0.1")
        .author("Rg .<daimaldd@gmail.com>")
        .about("lite bitcoin implements")
        .arg(Arg::with_name("wallets").long("config").default_value(
            "default_wallet.json",
        ))
        .subcommand(
            SubCommand::with_name("new")
                .about("new a bitcoin wallet")
                .arg(
                    Arg::with_name("force")
                        .short("f")
                        .long("force")
                        .default_value("false"),
                ),
        )
        .subcommand(SubCommand::with_name("open").about("open wallet"))
        .subcommand(
            SubCommand::with_name("create_blockchain")
                .about("recreate a new block chain")
                .arg(
                    Arg::with_name("store")
                        .long("store")
                        .default_value("/tmp/block_chain")
                        .value_name("STORE"),
                )
                .arg(
                    Arg::with_name("address")
                        .short("addr")
                        .long("address")
                        .value_name("ADDRESS"),
                ),
        )
        .get_matches();
    run(matches);
}

fn run(matches: ArgMatches) -> Result<(), String> {
    let config = matches.value_of("wallets").unwrap();
    match matches.subcommand() {
        ("new", Some(m)) => {
            info!(LOG, "wallet store {:?}", config);
            run_new(m, config);
            Ok(())
        }
        ("open", Some(m)) => {
            info!(LOG, "wallet store {:?}", config);
            run_open(m, config);
            Ok(())
        }
        ("create_blockchain", Some(m)) => {
            run_create_blockchain(m);
            Ok(())
        }
        _ => Ok(()),
    }
}

fn run_new(matches: &ArgMatches, wallet: &str) {
    let force = matches.value_of("force").unwrap().parse::<bool>().unwrap();
    cli::create_wallet(wallet.to_owned(), force);
}

fn run_open(matches: &ArgMatches, wallet: &str) {
    cli::open_wallet(wallet.to_owned());
}

fn run_create_blockchain(matches: &ArgMatches) {
    let store = matches.value_of("store").unwrap();
    let address = matches.value_of("address").unwrap();
    debug!(LOG, "address: {}, store: {}", address, store);
    cli::create_blockchain(address.to_owned(), store.to_owned()).unwrap();
}
