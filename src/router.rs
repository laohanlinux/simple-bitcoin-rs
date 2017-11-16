extern crate rocket;
extern crate io_context;
extern crate threadpool;

use self::io_context::Context;

use blockchain::BlockChain;
use server;
use transaction;
use utxo_set;

use std::sync::{Arc, Mutex};
use std::collections::HashMap;

pub struct BlockState {
    pub bc: Arc<BlockChain>,
    pub utxos: Arc<Mutex<utxo_set::UTXOSet>>,
    pub known_nodes: Arc<Mutex<Vec<String>>>,
    pub mining_address: Arc<String>,
    pub block_in_transit: Arc<Mutex<Vec<Vec<u8>>>>,
    pub mem_pool: Arc<Mutex<HashMap<String, transaction::Transaction>>>,
    pub local_node: Arc<String>,
    pub ctx: Context,
}

impl BlockState {
    pub fn new(
        bc: BlockChain,
        local_node: String,
        central_node: String,
        mining_address: String,
    ) -> BlockState {
        let arc_bc = Arc::new(bc);
        let utxo_set = utxo_set::UTXOSet::new(arc_bc.clone());
        utxo_set.reindex();
        let mut ctx = Context::background();
        BlockState {
            bc: arc_bc,
            utxos: Arc::new(Mutex::new(utxo_set)),
            known_nodes: Arc::new(Mutex::new(vec![central_node, local_node.clone()])),
            mining_address: Arc::new(mining_address),
            block_in_transit: Arc::new(Mutex::new(vec![])),
            mem_pool: Arc::new(Mutex::new(HashMap::new())),
            local_node: Arc::new(local_node),
            ctx: Context::background(),
        }
    }
}

pub fn init_router(addr: &str, port: u16, block_chain: BlockState) {
    let mut conf = rocket::Config::new(rocket::config::Environment::Production)
        .expect("invalid config");
    conf.set_address(addr).unwrap();
    conf.set_port(port);
    rocket::Rocket::custom(conf, true)
        .manage(block_chain)
        .mount("/", routes![server::handle_addr])
        .mount("/", routes![server::handle_get_blocks])
        .mount("/", routes![server::handle_inv])
        .mount("/", routes![server::handle_tx])
        .mount("/", routes![server::handle_version])
        .mount("/", routes![server::handle_block])
        .mount("/", routes![server::handle_get_block_data])
        .mount("/", routes![server::handle_generate_secrectkey])
        .mount("/", routes![server::handle_valid_pubkey])
        .mount("/", routes![server::handle_transfer])
        .launch();
}
