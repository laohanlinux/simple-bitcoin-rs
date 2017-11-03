extern crate serde_json;
extern crate rocket;
extern crate rocket_contrib;
extern crate lazy_static;

use self::rocket_contrib::{Json, Value};
use self::rocket::{Rocket, State};
use self::rocket::local::Client;
use self::rocket::http::ContentType;
use self::rocket::config::{Config, Environment};

use transaction::Transaction;
use log::*;
use blockchain::BlockChain;
use command::*;

use std::sync::{Arc, Mutex};
use std::collections::HashMap;

lazy_static!{
    static ref KNOWN_NODES: Mutex<Vec<String>> = Mutex::new(vec!["localhost:3000".to_owned()]);
    static ref MINING_ADDRESS: &'static str = "";
    static ref BLOCKS_IN_TRANSIT: Vec<Vec<u8>> = vec![];
    static ref MEMPOOL: HashMap<String, Transaction> = HashMap::new();
}

#[post("/addr", format = "application/json", data = "<addrs>")]
pub fn handle_addr(addrs: Json<Addr>) -> Json<Value> {
    {
        let mut knownNodes = KNOWN_NODES.lock().unwrap();
        knownNodes.append(&mut addrs.into_inner().addr_list);
        info!(LOG, "There are {} known nodes now", knownNodes.len());
    }
    request_blocks();
    ok_json()
}


// TODO
fn request_blocks() {
    let nodes = KNOWN_NODES.lock().unwrap();
    for node in nodes.iter() {
        send_get_block(node.clone())
    }
}

fn ok_json() -> Json<Value>{
    Json(json!({"status": "ok"}))
}

fn bad_json() -> Json<Value> {
    Json(json!({"status": "bad"}))
}

//// command
//fn send_get_block(address: String) {
//    let request = get_block { add_from: address.clone() };
//    let data = serde_json::to_vec(&request).unwrap();
//    let res = send_data(format!("{}/get_blocks", address.clone()), data);
//    if res.is_err() {
//        error!(LOG, "http request error {:?}", res.err());
//    } else {
//        debug!(
//            LOG,
//            "{} request success, return value {:?}",
//            address,
//            String::from_utf8_lossy(&res.unwrap())
//        );
//    }
//}

fn send_get_block(address: String) {
    let request = GetBlock{add_from: address};
    let data = serde_json::to_vec(&request).unwrap();
    let res = send_data()
}

fn send_data(address: String, path: Stirng, data: Vec<u8>) -> Result<Vec<u8>, String> {
    match rocket_post(address,path, data) {
        Some(data) => Ok(data),
        None => Err("data is nil")
    }
}

fn rocket_post(address: String, Path: String, data: Vec<u8>) -> Option<Vec<u8>> {
    let mut client = Client::new(rocket::ignite()).expect("valid rocket client");
    let req = client.post(Path)
        .header(ContentType::JSON)
        .remote(address.parse().unwrap())
        .body(data);
    let resp = req.dispatch();
    resp.body_bytes()
}