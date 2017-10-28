extern crate shio;
extern crate lazy_static;
extern crate serde;
extern crate serde_json;
extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate slog;
extern crate slog_term;

use self::shio::prelude::*;
use self::serde_json::Error;
use self::hyper::Client;
use self::hyper::{Method, Request};
use self::hyper::header::{ContentLength, ContentType};
use self::futures::{Future, Stream};
use self::tokio_core::reactor::Core;

use std::sync::Mutex;
use std::io;
use std::io::Write;
use std::io::prelude::*;
use std::collections::HashMap;
use std::borrow::Cow;

use transaction::Transaction;
use log::*;

lazy_static!{
    static ref KNOWN_NODES: Mutex<Vec<String>> = Mutex::new(vec!["localhost:3000".to_owned()]);
    static ref MINING_ADDRESS: &'static str = "";
    static ref BLOCKS_IN_TRANSIT: Vec<Vec<u8>> = vec![];
    static ref MEMPOOL: HashMap<String, Transaction> = HashMap::new();
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
    let res: Result<addr, Error> = serde_json::from_slice(&body);
    if res.is_err() {
        return Response::with(format!("{}", res.unwrap_err()));
    }

    Response::with("")
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

fn request_blocks() {
    let nodes = KNOWN_NODES.lock().unwrap();
    for node in nodes.iter() {}
}

fn send_get_block(address: String) {
    let request = get_block { add_from: address.clone() };
    let data = serde_json::to_vec(&request).unwrap();
    let res = send_data(format!("{}/get_block", address.clone()), data);
    if res.is_err() {
        error!(LOG, "http request error {:?}", res.err());
    } else {
        debug!(
            LOG,
            "{} request success, return value {:?}",
            address,
            String::from_utf8_lossy(&res.unwrap())
        );
    }
}

fn send_data(address: String, data: Vec<u8>) -> Result<Vec<u8>, hyper::Error> {
    let uri = format!("http://{}", address).parse()?;
    let mut core = Core::new().unwrap();
    let client = Client::new(&core.handle());
    let mut req = Request::new(Method::Post, uri);
    req.headers_mut().set(ContentType::json());
    req.headers_mut().set(ContentLength(data.len() as u64));
    req.set_body(data);
    let post = client.request(req).and_then(|res| res.body().concat2());
    let data = core.run(post)?;
    Ok(data.to_vec())
}
