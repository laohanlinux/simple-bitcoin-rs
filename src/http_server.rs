extern crate shio;
extern crate lazy_static;
extern crate serde;
extern crate serde_json;

use self::shio::prelude::*;

use std::sync::Mutex;
use std::io;
use std::io::prelude::*;

lazy_static!{
    static ref KNOWN_NODES: Mutex<Vec<String>> = Mutex::new(vec!["localhost:3000".to_owned()]);
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct addr {
    addr_list: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct block {
    add_from: String,
    block: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct get_block {
    add_from: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct get_data {
    add_from: String,
    data_type: String,
    id: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct inv {
    add_from: String,
    inv_type: String,
    items: Vec<Vec<u8>>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct tx {
    add_from: String,
    transaction: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct verzion {
    version: isize,
    best_hight: isize,
    addr_from: String,
}

fn handle_addr(ctx: Context) -> Response {
    let mut body = Vec::new();
    if ctx.body().read_to_end(&mut body).is_err() {
        return Response::with("shift");
    }
    Response::with("Hello")
}

pub fn node_is_known(addr: String) -> bool {
    let nodes = KNOWN_NODES.lock().unwrap();
    for node in nodes.iter() {
        if node.to_owned() == addr {
            return true;
        }
    }
    false
}
