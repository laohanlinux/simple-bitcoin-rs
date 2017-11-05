extern crate serde_json;
extern crate rocket;
extern crate rocket_contrib;
extern crate lazy_static;

use self::rocket_contrib::{Json, Value};
use self::rocket::{State};
use self::rocket::local::Client;
use self::rocket::http::ContentType;
use self::rocket::config::{ Environment};

use transaction::Transaction;
use log::*;
use blockchain::BlockChain;
use command::*;
use router;
use util;
use block;

#[post("/addr", format = "application/json", data = "<addrs>")]
pub fn handle_addr(state: rocket::State<router::BlockState>, addrs: Json<Addr>) -> Json<Value> {
    let mut node_list = vec![];
    {
        let known_nodes_lock = state.known_nodes.clone();
        let mut know_nodes = known_nodes_lock.lock().unwrap();
        know_nodes.append(&mut addrs.into_inner().addr_list);
        info!(LOG, "There are {} known nodes now", know_nodes.len());
        node_list = know_nodes.clone();
    }
    request_blocks(node_list);
    ok_json!()
}

#[get("/get_blocks")]
pub fn handle_get_blocks(
    blocks: rocket::State<router::BlockState>
) -> Json<Value> {
    let bc = blocks.bc.clone();
    let hashes: Vec<Vec<u8>> = bc.get_block_hashes();
    let hashes: Vec<String> = hashes.into_iter()
        .map(|item| util::encode_hex(item))
        .collect();
    ok_data_json!(hashes)
}

// TODO before add block, check it's valid
#[post("/block", format = "application/json", data = "<block_data>")]
pub fn handle_block(state: rocket::State<router::BlockState>, block_data: Json<Block>) -> Json<Value> {
    let bc = state.bc.clone();
    let new_block = block::Block::deserialize_block(&block_data.block);
    let block_hash = new_block.hash.clone();
    bc.add_block(new_block.clone());
    info!(LOG, "added block {}", util::encode_hex(&block_hash));

    // TODO why do it in that.
    let block_in_transit = state.block_in_transit.clone();
    {
        let mut bc_in_transit = block_in_transit.lock().unwrap();
        if bc_in_transit.len() > 0 {
            let block_hash = bc_in_transit[0].clone();
            send_get_data(&block_data.add_from, "block".to_owned(), block_hash);
            *bc_in_transit = bc_in_transit[1..].to_vec();
        }else {
            // update utxo
            let utxo = state.utxos.clone();
            let utxos = utxo.lock().unwrap();
            utxos.update(&new_block);
        }
    }
    
    ok_data_json!("")
}

// TODO
fn request_blocks(know_nodes: Vec<String>) {
    for node in &know_nodes {
        send_get_block(node)
    }
}

fn send_get_data(address: &str, kind: String, id: Vec<u8>) {
    let data = GetData{
        add_from: address.to_string(),
        data_type: kind,
        id: id,
    };
    let data = serde_json::to_vec(&data).unwrap();
    let path = format!("{}/get_data", address);
    let res = send_data(address, &path, &data);
    if res.is_ok() {
        info!(LOG, "send get data successfully.");
    }
}


// path => /get_blocks
fn send_get_block(address: &str) {
    let request = GetBlock { add_from: address.to_string() };
    let data = serde_json::to_vec(&request).unwrap();
    let path = format!("{}/get_blocks", address);
    let res = send_data(address, &path, &data);
    if res.is_err() {
        error!(LOG, "http request error {:?}", res.err());
    } else {
        debug!(LOG, "node {}: request({}) success", &address, &path);
        debug!(LOG, "node {}: blocks: {}", &address, String::from_utf8_lossy(&res.unwrap()));
    }
}

fn send_data(address: &str, path: &str, data: &[u8]) -> Result<Vec<u8>, String> {
    match rocket_post(address, path, data) {
        Some(data) => Ok(data),
        None => Err("data is nil".to_owned()),
    }
}

fn rocket_post(address: &str, Path: &str, data: &[u8]) -> Option<Vec<u8>> {
    let client = Client::new(rocket::ignite()).expect("valid rocket client");
    let req = client
        .post(Path)
        .header(ContentType::JSON)
        .remote(address.parse().unwrap())
        .body(data);
    let mut resp = req.dispatch();
    resp.body_bytes()
}
