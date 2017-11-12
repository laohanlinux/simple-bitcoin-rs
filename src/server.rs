extern crate serde_json;
extern crate rocket;
extern crate rocket_contrib;
extern crate lazy_static;
extern crate tokio_core;
extern crate tokio_request;
extern crate url;

use self::rocket_contrib::{Json, Value};
use self::tokio_core::reactor::Core;
use self::tokio_request::str::post;

use std::sync::Arc;
use std::collections::HashMap;

use transaction::Transaction;
use log::*;
use blockchain::BlockChain;
use command::*;
use router;
use util;
use block;
use utxo_set;
use wallet;

#[get("/wallet/generate_secretkey")]
pub fn handle_generate_secrectkey(state: rocket::State<router::BlockState>) -> Json<Value> {
    let data = wallet::Wallet::new().to_btc_pair();
    ok_data_json!(data)
}

#[get("/wallet/valid_pubkey/<pubkey>")]
pub fn handle_valid_pubkey(
    state: rocket::State<router::BlockState>,
    pubkey: String,
) -> Json<Value> {
    if wallet::Wallet::validate_address(pubkey.clone()) {
        ok_json!()
    } else {
        bad_data_json!(format!("{} is invalid btc address", pubkey))
    }
}

#[post("/wallet/transfer", format = "application/json", data = "<transfer>")]
pub fn handle_transfer(
    state: rocket::State<router::BlockState>,
    transfer: Json<Transfer>,
) -> Json<Value> {

    if !wallet::Wallet::validate_address(transfer.from.clone()) {
        return bad_data_json!("ERROR: From's address is not valid".to_owned());
    }
    if !wallet::Wallet::validate_address(transfer.to.clone()) {
        return bad_data_json!("ERROR: To's address is not valid".to_owned());
    }

    let from_wallet = wallet::Wallet::recover_wallet(&util::decode_hex(&transfer.secret_key));
    if from_wallet.is_err() {
        return bad_data_json!(from_wallet.err());
    }
    let from_wallet = from_wallet.unwrap();
    let (to, amount) = (&transfer.to, &transfer.amount);
    let bc = state.bc.clone();
    let utxo = utxo_set::UTXOSet::new(bc.clone());

    //let result =  Transaction::new_utxo_transaction(&from_wallet, to.clone(), amount as isize, &utxo);
    // TODO broabow all node, ( node will add the transaction into transaction memory pool)

    /*let new_block = if mine_now {
        let cbtx = transaction::Transaction::new_coinbase_tx(from.clone(), "".to_owned());
        let txs = vec![cbtx, result];
        let new_block = block_chain.clone().mine_block(&txs);
        Some(new_block)
    } else {
        None
    };
    if new_block.is_none() {
        return Err("ERROR: generate block fail".to_owned());
    }
    utxo.update(&new_block.unwrap());
    info!(LOG, "{:?} send {} to {:?}", from, amount, to);
    */
    ok_json!()
}

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

// sync data
#[post("/get_data", format = "application/json", data = "<block_data>")]
pub fn handle_get_block_data(
    state: rocket::State<router::BlockState>,
    block_data: Json<GetData>,
) -> Json<Value> {
    let get_type = &block_data.data_type;
    let bc = state.bc.clone();
    let block_hash = &block_data.id;
    if get_type == "block" {
        let block = bc.get_block(&util::decode_hex(&block_data.id));
        if block.is_none() {
            return bad_json!();
        }
        let local_node = state.local_node.clone();
        send_block(&block_data.add_from, &local_node, block.unwrap());
    }
    if get_type == "tx" {
        let txid = util::encode_hex(&block_data.id);
        let tx = {
            let mem_pool = state.mem_pool.clone();
            let mem_pool = mem_pool.lock().unwrap();
            mem_pool.get(&txid).unwrap().clone()
        };
        // TODO
        let local_node = state.local_node.clone();
        send_tx(&block_data.add_from, &local_node, &tx);
        // TODO delete mempool, txid
    }

    // TODO
    ok_json!()
}

#[post("/tx", format = "application/json", data = "<tx>")]
pub fn handle_tx(state: rocket::State<router::BlockState>, tx: Json<TX>) -> Json<Value> {
    let txdata = &tx.transaction;
    let ts: Transaction = serde_json::from_slice(txdata).unwrap();
    let txid = util::encode_hex(&ts.id);
    // add new transaction into mempool
    let mem_pool = state.mem_pool.clone();
    let mut mem_pool = mem_pool.lock().unwrap();
    mem_pool.entry(txid).or_insert(ts.clone());

    // local node addr
    let local_node = state.local_node.clone();

    // central node
    let known_nodes = state.known_nodes.clone();
    let ref known_nodes = {
        let know_nodes = known_nodes.lock().unwrap();
        know_nodes.clone()
    };
    // TODO i don't knonw why do that
    if local_node.to_lowercase() == known_nodes[0].to_lowercase() {
        let txid = &ts.id;
        for node in known_nodes {
            send_inv(node, "tx", vec![txid.to_vec()]);
        }
    } else {
        let mining_addr = state.mining_address.clone();
        loop {
            if mem_pool.len() >= 2 && mining_addr.len() > 0 {
                // mine transactions
                let mut txs = vec![];
                let bc = state.bc.clone();
                let cbtx = Transaction::new_coinbase_tx(mining_addr.to_lowercase(), "".to_owned());
                txs.push(cbtx);
                for (txid, ts) in &*mem_pool {
                    if bc.verify_transaction(ts) {
                        txs.push(ts.clone());
                    }
                }
                if txs.len() <= 1 {
                    return ok_json!();
                }

                let new_block = bc.mine_block(&txs);
                let utxo = utxo_set::UTXOSet::new(bc);
                //reset unspend transations
                // TODO use update instead of reindex
                utxo.reindex();
                info!(
                    LOG,
                    "mining a new block, hash is {}",
                    util::encode_hex(&new_block.hash)
                );

                // delete dirty transaction
                for ts in &txs {
                    mem_pool.remove(&util::encode_hex(&ts.id));
                }

                // notify other nodes
                // filter local node
                &known_nodes.into_iter().for_each(|node| if *node !=
                    local_node.to_string()
                {
                    send_inv(&node, "block", vec![new_block.hash.clone()])
                });
            }
            if mem_pool.len() <= 0 {
                break;
            }
        }
    }
    // TODO
    ok_json!()
}

// sync block, return all block hashes
#[get("/get_blocks")]
pub fn handle_get_blocks(blocks: rocket::State<router::BlockState>) -> Json<Value> {
    let bc = blocks.bc.clone();
    let hashes: Vec<Vec<u8>> = bc.get_block_hashes();
    let hashes: Vec<String> = hashes
        .into_iter()
        .map(|item| util::encode_hex(item))
        .collect();
    ok_data_json!(hashes)
}

#[post("/version", format = "application/json", data = "<version>")]
pub fn handle_version(
    state: rocket::State<router::BlockState>,
    version: Json<Version>,
) -> Json<Value> {
    let bc = state.bc.clone();
    let my_best_height = bc.get_best_height();
    let foreigner_best_height = version.best_height;
    if my_best_height < foreigner_best_height {
        // TODO
        send_get_block(&version.addr_from);
    } else if my_best_height > foreigner_best_height {
        let local_node = state.local_node.clone();
        send_version(&version.addr_from, &local_node, bc);
    }
    ok_json!()
}

#[post("/inv", format = "application/json", data = "<inv>")]
pub fn handle_inv(state: rocket::State<router::BlockState>, inv: Json<Inv>) -> Json<Value> {
    info!(
        LOG,
        "Received inventory with {} {}",
        inv.items.len(),
        inv.inv_type
    );
    let inv_type = &inv.inv_type;
    if inv_type == "block" {
        let block_in_transit = state.block_in_transit.clone();
        let mut block_in_transit = block_in_transit.lock().unwrap();
        *block_in_transit = inv.items.clone();
        let block_hash = inv.items[0].clone();
        send_get_data(&inv.add_from, "block".to_owned(), block_hash.clone());

        let mut new_in_transit: Vec<Vec<u8>> = vec![];
        for b in &*block_in_transit {
            if !util::compare_slice_u8(&b, &block_hash) {
                new_in_transit.push(b.clone());
            }
        }
        // reset blocks in transit
        *block_in_transit = new_in_transit;
    }
    if inv_type == "tx" {
        let txid = inv.items[0].clone();
        let mem_pool = state.mem_pool.clone();
        let mem_pool = mem_pool.lock().unwrap();
        if mem_pool.get(&util::encode_hex(&txid)).is_none() {
            send_get_data(&inv.add_from, "tx".to_owned(), txid);
        }
    }
    ok_json!()
}

// TODO before add block, check it's valid
#[post("/block", format = "application/json", data = "<block_data>")]
pub fn handle_block(
    state: rocket::State<router::BlockState>,
    block_data: Json<Block>,
) -> Json<Value> {
    info!(LOG, "do block handle");
    let bc = state.bc.clone();
    let new_block = block::Block::try_deserialize_block(&block_data.block);
    if new_block.is_err() {
        return bad_data_json!(new_block.err().unwrap());
    }
    let new_block = new_block.ok().unwrap();
    let block_hash = new_block.hash.clone();
    if bc.get_block(&block_hash).is_some() {
        warn!(
            LOG,
            "block {} has exists, ignore it",
            util::encode_hex(&block_hash)
        );
        return ok_json!();
    }
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
        } else {
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

fn send_get_data(addr: &str, kind: String, id: Vec<u8>) {
    let data = GetData {
        add_from: addr.to_string(),
        data_type: kind,
        id: util::encode_hex(&id),
    };
    let data = serde_json::to_vec(&data).unwrap();
    let path = format!("{}/get_data", addr);
    let res = send_data(addr, &path, &data);
    print_http_result(addr.to_string() + "/get_data", res);
}

// path => /inv
fn send_inv(addr: &str, kind: &str, items: Vec<Vec<u8>>) {
    let inventory = Inv {
        add_from: addr.to_owned(),
        inv_type: kind.to_owned(),
        items: items,
    };
    let data = serde_json::to_vec(&inventory).unwrap();
    let res = send_data(addr, "/inv", &data);
    print_http_result(addr.to_string() + "/inv", res);
}

// path => /tx
fn send_tx(addr: &str, local_node: &str, block: &Transaction) {
    let data = serde_json::to_vec(&TX {
        add_from: addr.to_owned(),
        transaction: serde_json::to_vec(block).unwrap(),
    }).unwrap();
    let res = send_data(addr, "/tx", &data);
    print_http_result(addr.to_string() + "/tx", res);
}

// path => /block
fn send_block(addr: &str, local_node: &str, block: block::Block) {
    let data = serde_json::to_vec(&Block {
        add_from: addr.to_owned(),
        block: serde_json::to_vec(&block).unwrap(),
    }).unwrap();
    let res = send_data(local_node, "/block", &data);
    print_http_result(local_node.to_string() + "/block", res);
}

// path => /get_blocks
fn send_get_block(addr: &str) {
    let request = GetBlock { add_from: addr.to_string() };
    let data = serde_json::to_vec(&request).unwrap();
    let path = format!("{}/get_blocks", addr);
    let res = send_data(addr, &path, &data);
    print_http_result(addr.to_string() + "/get_blocks", res);
}

// send local node version to remote addr
// path => /version
fn send_version(addr: &str, local_node: &str, bc: Arc<BlockChain>) {
    let best_height = bc.get_best_height();
    let version = Version::new(NODE_VERSION, best_height, local_node.to_owned());
    let res = send_data(addr, "/version", &serde_json::to_vec(&version).unwrap());
    print_http_result(addr.to_string() + "/version", res);
}

fn send_data(address: &str, path: &str, data: &[u8]) -> Result<Vec<u8>, String> {
    let (status, body) = tokio_http_post(address, path, data);
    if status != 200 {
        let msg = format!("status code is {}", status);
        return Err(msg);
    }
    match body {
        Some(data) => Ok(data),
        None => Err("data is nil".to_owned()),
    }
}

fn print_http_result(uri: String, res: Result<Vec<u8>, String>) {
    if res.is_ok() {
        info!(LOG, "send get data successfully, URI => {}", uri);
    } else {
        error!(
            LOG,
            "send get data fail, URI => {}, err: {:?}",
            uri,
            res.err().unwrap()
        );
    }
}

fn tokio_http_post(addr: &str, path: &str, data: &[u8]) -> (u16, Option<Vec<u8>>) {
    debug!(
        LOG,
        "address {}, path {}, data {}",
        addr,
        path,
        String::from_utf8_lossy(data)
    );
    let addr = format!("http://{}{}", addr, path);
    let mut evloop = Core::new().unwrap();
    let future = post(&addr)
        .header("content-type", "application/json")
        .body(data.to_vec())
        .send(evloop.handle());
    let result = evloop.run(future).expect("HTTP Request failed!");
    if result.is_success() == false {
        return (result.status_code(), None);
    }
    let body = result.body();
    (200, Some(body.to_vec()))
}
