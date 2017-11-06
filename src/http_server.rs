extern crate shio;

extern crate serde;
extern crate serde_json;
extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate slog;
extern crate slog_term;
extern crate validator;

use self::shio::prelude::*;
use self::shio::context::Key;
use self::serde_json::Error;
use self::hyper::{Client, Method, Request, StatusCode};
use self::hyper::header::{ContentLength, ContentType};
use self::futures::{Future, Stream};
use self::tokio_core::reactor::Core;

use self::validator::{Validate, ValidationError};

use std::sync::Mutex;
use std::io;
use std::io::Write;
use std::io::prelude::*;
use std::collections::HashMap;
use std::borrow::Cow;

//fn handle_addr(ctx: Context) -> Response {
//    let mut body = Vec::new();
//    if ctx.body().read_to_end(&mut body).is_err() {
//        return Response::with("shift");
//    }
//    let res: Result<addr, Error> = serde_json::from_slice(&body);
//    if res.is_err() {
//        return Response::with(format!("{}", res.unwrap_err()));
//    }
//
//    {
//        let mut knownNodes = KNOWN_NODES.lock().unwrap();
//        let mut list_addr = res.unwrap();
//        knownNodes.append(&mut list_addr.addr_list);
//        info!(LOG, "There are {} known nodes now", knownNodes.len());
//    }
//    request_blocks();
//    ok_response()
//}
//
//fn handle_get_blocks(ctx: Context) -> Response {
//    let mut body = Vec::new();
//    if ctx.body().read_to_end(&mut body).is_err() {
//        return bad_read_request_body();
//    }
//    let block = ctx.shared().get::<&BlockChain>();
//    let block_hash = block.get_block_hashes();
//    ok_response(&block_hash)
//}
//*/
//pub fn node_is_known(addr: String) -> bool {
//    let nodes = KNOWN_NODES.lock().unwrap();
//    for node in nodes.iter() {
//        if node.to_owned() == addr {
//            return true;
//        }
//    }
//    false
//}
//
//
//
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
//
//fn send_inv(address: String, kind: String, item: Vec<Vec<u8>>) {
//    let inventory = inv {
//        add_from: address.clone(),
//        inv_type: kind,
//        items: item,
//    };
//
//    let data = serde_json::to_vec(&inventory).unwrap();
//    let res = send_data(address.clone(), data.clone());
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
//
