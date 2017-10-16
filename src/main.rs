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

#[macro_use]
extern crate bigint;

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

const STORE: &str = "/tmp/block_chain";
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
                        .default_value(STORE)
                        .value_name("STORE"),
                )
                .arg(
                    Arg::with_name("address")
                        .short("addr")
                        .long("address")
                        .value_name("ADDRESS"),
                ),
        )
        .subcommand(
            SubCommand::with_name("add_wallet")
                .about("add a new wallet")
                .arg(
                    Arg::with_name("store")
                        .long("store")
                        .default_value(STORE)
                        .value_name("STORE")
                ),
        )
        .subcommand(
            SubCommand::with_name("print")
                .about("print all the block")
                .arg(
                    Arg::with_name("store")
                        .long("store")
                        .default_value(STORE)
                        .value_name("STORE"),
                ),
        )
        .subcommand(
            SubCommand::with_name("reindex")
                .about("rebuild utxo")
                .arg(
                    Arg::with_name("store")
                        .long("store")
                        .default_value(STORE)
                        .value_name("STORE"),
                )
                .arg(
                    Arg::with_name("address")
                        .short("addr")
                        .long("address")
                        .value_name("ADDRESS"),
                ),
        )
        .subcommand(
            SubCommand::with_name("balance")
                .about("get accout's balances")
                .arg(Arg::with_name("store").long("store").default_value(STORE))
                .arg(
                    Arg::with_name("address")
                        .long("address")
                        .short("addr")
                        .value_name("ADDRESS"),
                ),
        )
        .subcommand(
            SubCommand::with_name("send")
                .about("send money...")
                .arg(Arg::with_name("store").long("store").default_value(
                    STORE,
                ))
                .arg(
                    Arg::with_name("wallet")
                        .long("wallet")
                        .default_value("default_wallet.json")
                        .value_name("wallet"),
                )
                .arg(Arg::with_name("from").long("from").value_name("FROM"))
                .arg(Arg::with_name("to").long("to").value_name("TO"))
                .arg(Arg::with_name("amount").long("amount").value_name("amount"))
                .arg(
                    Arg::with_name("mine")
                        .long("mine")
                        .default_value("false")
                        .value_name("mine"),
                ),
        )
        .get_matches();
    if let Err(e) = run(matches) {
        error!(LOG, "{}", e);
    }
}

fn run(matches: ArgMatches) -> Result<(), String> {
    let config = matches.value_of("wallets").unwrap();
    match matches.subcommand() {
        ("new", Some(m)) => {
            info!(LOG, "wallet store {:?}", config);
            run_new(m, config);
            Ok(())
        }
        ("add_wallet", Some(m)) => {
            run_add_wallet(m, config);
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
        ("print", Some(m)) => {
            run_print(m);
            Ok(())
        }
        ("reindex", Some(m)) => {
            run_reindex(m);
            Ok(())
        }
        ("balance", Some(m)) => {
            run_get_balance(m);
            Ok(())
        }
        ("send", Some(m)) => {
            run_send(m);
            Ok(())
        }
        _ => Ok(()),
    }
}

fn run_new(matches: &ArgMatches, wallet: &str) {
    let force = matches.value_of("force").unwrap().parse::<bool>().unwrap();
    cli::create_wallet(wallet.to_owned(), force);
}

fn run_add_wallet(matches: &ArgMatches, wallet: &str) {
    cli::add_wallet(wallet.to_owned());
}

fn run_open(matches: &ArgMatches, wallet: &str) {
    cli::open_wallet(wallet.to_owned());
}

fn run_create_blockchain(matches: &ArgMatches) {
    let store = matches.value_of("store").unwrap();
    let address = matches.value_of("address").unwrap();
    cli::create_blockchain(address.to_owned(), store.to_owned()).unwrap();
}

fn run_print(matches: &ArgMatches) {
    let store = matches.value_of("store").unwrap();
    cli::print_chain(store.to_owned()).unwrap();
}

fn run_reindex(matches: &ArgMatches) {
    let store = matches.value_of("store").unwrap();
    cli::reindex_utxo(store.to_owned()).unwrap();
}

fn run_get_balance(matches: &ArgMatches) {
    let store = matches.value_of("store").unwrap();
    let address = matches.value_of("address").unwrap();
    cli::get_balance(address.to_owned(), store.to_owned()).unwrap();
}

fn run_send(matches: &ArgMatches) {
    let store = matches.value_of("store").unwrap();
    let wallet_store = matches.value_of("wallet").unwrap();
    let from = matches.value_of("from").unwrap();
    let to = matches.value_of("to").unwrap();
    let amount = matches
        .value_of("amount")
        .unwrap()
        .parse::<isize>()
        .unwrap();
    let mine = matches.value_of("mine").unwrap().parse::<bool>().unwrap();
    cli::send(
        from.to_owned(),
        to.to_owned(),
        amount,
        wallet_store.to_owned(),
        store.to_owned(),
        mine,
    ).unwrap();
}
