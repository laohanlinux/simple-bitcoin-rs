extern crate serde_json;
extern crate rocket;
extern crate rocket_contrib;
extern crate lazy_static;

use self::rocket_contrib::{Json, Value};
use self::rocket::{Rocket, State};
use self::rocket::local::Client;
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

fn send_data(address: String, data: Vec<u8>) -> Result<Vec<u8>, hyper::Error> {
    let client = Client::new(Rocket::ignite()).unwrap();

//    let uri = format!("http://{}", address).parse()?;
//    let mut core = Core::new().unwrap();
//    let client = Client::new(&core.handle());
//    let mut req = Request::new(Method::Post, uri);
//    req.headers_mut().set(ContentType::json());
//    req.headers_mut().set(ContentLength(data.len() as u64));
//    req.set_body(data);
//    let post = client.request(req).and_then(|res| res.body().concat2());
//    let data = core.run(post)?;
//    Ok(data.to_vec())
}

fn rocket(address: String) -> Rocket {
    let mut rocket = Config::new(Environment::Production).expect("bad rocket configure");
    
}