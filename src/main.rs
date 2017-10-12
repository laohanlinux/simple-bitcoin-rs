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

extern crate clap;
use clap::{Arg, App, SubCommand};

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

#[derive(ArgKey, SubCommand)]
enum SubCommands {

}

fn main() {
    let matches = App::new("My Super Program")
        .version("1.0")
        .author("Rg .<daimaldd@gmail.com>")
        .about("lite bitcoin")
        .arg(Arg::with_name("config").short("c").long("config"))
        .subcommand(
            SubCommand::with_name("new").arg(
                Arg::with_name("force")
                    .short("f")
                    .long("force")
                    .takes_value(true),
            ),
        )
        .subcommand(SubCommand::with_name("open"))
        .get_matches();

    let config = matches.value_of("config").unwrap_or("default_wallet.json");
    info!(LOG, "{:?}", force);
    info!(LOG, "wallet store {:?}", config);
    match matches.subcommand_name() {
        Some("new") => {
            cli::create_wallet(config.to_owned(), force);
            let force = if let Some(ref matches) = matches.subcommand_matches("new") {
                matches.value_of("force")
        .unwrap()//("false")
        .to_owned()
        .parse::<bool>()
        .unwrap()
            };

        }
        None => error!(LOG, "No subcommand was used"),
        _ => error!(LOG, "Some other subcommand was used"),
    }
}
