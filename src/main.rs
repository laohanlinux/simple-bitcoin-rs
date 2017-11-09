#[macro_use]
extern crate slog;

extern crate clap;
extern crate simple_bitcoin_rs;

use clap::{Arg, App, SubCommand, ArgMatches};
use simple_bitcoin_rs::log::*;
use simple_bitcoin_rs::cli;

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
                        .value_name("STORE"),
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
            SubCommand::with_name("balances")
                .about("get accout's balances")
                .arg(
                    Arg::with_name("wallet_store")
                        .long("wallet_store")
                        .default_value("default_wallet.json"),
                )
                .arg(Arg::with_name("store").long("store").default_value(STORE)),
        )
        .subcommand(
            SubCommand::with_name("utxo")
                .about("get transaction utxo")
                .arg(Arg::with_name("txid").long("txid").value_name("txid"))
                .arg(Arg::with_name("store").long("store").default_value(STORE)),
        )
        .subcommand(
            SubCommand::with_name("utxos")
                .about("get transaction utxos")
                .arg(Arg::with_name("store").long("store").default_value(STORE)),
        )
        .subcommand(
            SubCommand::with_name("server")
                .about("start a p2p node")
                .arg(Arg::with_name("store").long("store").default_value(STORE))
                .arg(
                    Arg::with_name("addr")
                        .long("addr")
                        .value_name("ADDR")
                        .default_value("127.0.0.1"),
                )
                .arg(
                    Arg::with_name("port")
                        .long("port")
                        .value_name("PORT")
                        .default_value("8821"),
                ),
        )
        .subcommand(
            SubCommand::with_name("send")
                .about("send money...")
                .arg(Arg::with_name("store").long("store").default_value(STORE))
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
        .subcommand(
            SubCommand::with_name("list_transactions")
                .about("list all transactions")
                .arg(Arg::with_name("store").long("store").default_value(STORE)),
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
            Ok(run_new(m, config))
        }
        ("add_wallet", Some(m)) => Ok(run_add_wallet(m, config)),
        ("open", Some(m)) => {
            info!(LOG, "wallet store {:?}", config);
            Ok(run_open(m, config))
        }
        ("create_blockchain", Some(m)) => Ok(run_create_blockchain(m)),
        ("print", Some(m)) => Ok(run_print(m)),
        ("reindex", Some(m)) => Ok(run_reindex(m)),
        ("balance", Some(m)) => Ok(run_get_balance(m)),
        ("balances", Some(m)) => Ok(run_get_balances(m)),
        ("utxo", Some(m)) => Ok(run_get_utxo(m)),
        ("utxos", Some(m)) => Ok(run_get_utxos(m)),
        ("list_transactions", Some(m)) => Ok(run_list_transactions(m)),
        ("send", Some(m)) => Ok(run_send(m)),
        ("server", Some(m)) => Ok(run_server(m)),
        _ => Ok(()),
    }


}

fn run_server(mathes: &ArgMatches) {
    let store = mathes.value_of("store").unwrap();
    let addr = mathes.value_of("addr").unwrap();
    let port = mathes.value_of("port").unwrap().parse::<u16>().unwrap();
    cli::start_server(store.to_owned(), &addr, port);
}

fn run_new(matches: &ArgMatches, wallet: &str) {
    let force = matches.value_of("force").unwrap().parse::<bool>().unwrap();
    cli::create_wallet(wallet.to_owned(), force);
}

fn run_add_wallet(_: &ArgMatches, wallet: &str) {
    cli::add_wallet(wallet.to_owned());
}

fn run_open(_: &ArgMatches, wallet: &str) {
    cli::open_wallet(wallet.to_owned());
}

fn run_create_blockchain(matches: &ArgMatches) {
    let store = matches.value_of("store").unwrap();
    let address = matches.value_of("address").unwrap();
    match cli::create_blockchain(address.to_owned(), store.to_owned()) {
        Ok(_) => {}
        Err(e) => println!("{}", e),
    }
}

fn run_print(matches: &ArgMatches) {
    let store = matches.value_of("store").unwrap();
    match cli::print_chain(store.to_owned()) {
        Err(e) => println!("{}", e),
        Ok(_) => {}
    }
}

fn run_reindex(matches: &ArgMatches) {
    let store = matches.value_of("store").unwrap();
    match cli::reindex_utxo(store.to_owned()) {
        Err(e) => println!("{}", e),
        _ => {}
    }
}

fn run_get_balance(matches: &ArgMatches) {
    let store = matches.value_of("store").unwrap();
    let address = matches.value_of("address").unwrap();
    match cli::get_balance(address.to_owned(), store.to_owned()) {
        Err(e) => println!("{}", e),
        _ => {}
    }
}

fn run_get_balances(matches: &ArgMatches) {
    let wallet_store = matches.value_of("wallet_store").unwrap();
    let store = matches.value_of("store").unwrap();
    match cli::get_balances(wallet_store.to_owned(), store.to_owned()) {
        Err(e) => print!("{}", e),
        _ => {}
    }
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
    match cli::send(
        from.to_owned(),
        to.to_owned(),
        amount,
        wallet_store.to_owned(),
        store.to_owned(),
        mine,
    ) {
        Ok(_) => {}
        Err(e) => println!("{}", e), 
    }
}

fn run_get_utxo(matches: &ArgMatches) {
    let store = matches.value_of("store").unwrap();
    let txid = matches.value_of("txid").unwrap();
    cli::get_utxo(txid.to_owned(), store.to_owned()).unwrap();
}

fn run_get_utxos(matches: &ArgMatches) {
    let store = matches.value_of("store").unwrap();
    cli::get_utxos(store.to_owned()).unwrap();
}

fn run_list_transactions(matches: &ArgMatches) {
    let store = matches.value_of("store").unwrap();
    cli::list_transactions(store.to_owned()).unwrap();
}
