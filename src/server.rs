extern crate serde_json;
extern crate rocket;
extern crate rocket_contrib;
extern crate lazy_static;
extern crate base_emoji;

use self::rocket::request::Form;
use self::rocket_contrib::{Json, Value};
use self::rocket::response::NamedFile;

use std::io;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use transaction::Transaction;
use log::*;
use blockchain::BlockChain;
use command::*;
use router;
use util;
use utxo_set;
use wallet;
use pool;
use block;

const MINING_SIZE: usize = 1;

#[get("/node/list")]
pub fn handle_node_list(state: rocket::State<router::BlockState>) -> Json<Value> {
    let known_nodes = state.known_nodes.clone();
    let known_nodes = known_nodes.lock().unwrap();
    ok_data_json!(known_nodes.clone())
}

#[get("/mempool/list")]
pub fn handle_mempool_list(state: rocket::State<router::BlockState>) -> Json<Value> {
    let mem_pool = &state.mem_pool.lock().unwrap().clone();
    ok_data_json!(mem_pool)
}

#[get("/test/download")]
pub fn handle_test_download_blocks(state: rocket::State<router::BlockState>) -> Json<Value> {
    let bc = &state.bc.lock().unwrap();
    let all_blocks = bc.download_blocks();

    ok_data_json!(all_blocks)
}

#[get("/test/blocks")]
pub fn handle_test_list_block(state: rocket::State<router::BlockState>) -> Json<Value> {
    let bc = &state.bc.lock().unwrap();
    let hashes = bc.test_block_hashes();
    ok_data_json!(hashes)
}

#[get("/test/last/block")]
pub fn handle_test_last_block(state: rocket::State<router::BlockState>) -> Json<Value> {
    let bc = &state.bc.lock().unwrap();
    ok_data_json!(bc.test_last_block())
}

#[get("/test/mempool/blocks")]
pub fn handle_test_mempool_blocks(state: rocket::State<router::BlockState>) -> Json<Value> {
    let mem_pool = state.mem_pool.lock().unwrap();
    let data = mem_pool.clone();
    let output:Vec<String> = data.into_iter().map(|(k, _)| k).collect();
    ok_data_json!(output)
}

#[get("/wallet/balance/<addr>")]
pub fn handle_balance(state: rocket::State<router::BlockState>, addr: String) -> Json<Value> {
    let bc = &state.bc.lock().unwrap();
    ok_data_json!(bc.balance(&addr))
}

#[get("/wallet/info/tx/<id>")]
pub fn handle_tx_info(state: rocket::State<router::BlockState>, id: String) -> Json<Value> {
    let bc = &state.bc.lock().unwrap();
    bc.tx(&id).map_or(bad_data_json!(format!("{} not found", id)), |ts| ok_data_json!(ts))
}

#[get("/wallet/info/block/<id>")]
pub fn handle_info_block(state: rocket::State<router::BlockState>, id: String) -> Json<Value> {
    let bc = &state.bc.lock().unwrap();
    bc.block(&id).map_or(bad_data_json!(format!("{} not found", id)), |block| {ok_data_json!(block)})
}

#[get("/wallet/utxos/unspend")]
pub fn handle_unspend_utxos(state: rocket::State<router::BlockState>) -> Json<Value> {
    let bc = &state.bc.lock().unwrap();
    ok_data_json!(bc.unspend_utxo())
}

#[get("/wallet/blocks")]
pub fn handle_list_block(state: rocket::State<router::BlockState>) -> Json<Value> {
    let block = &state.bc.lock().unwrap();
    let hashes = block.block_hashes();
    ok_data_json!(hashes)
}

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

#[post("/wallet/transfer", data = "<transfer>")]
pub fn handle_transfer(
    state: rocket::State<router::BlockState>,
    transfer: Form<Transfer>,
) -> Json<Value> {
    let transfer = transfer.into_inner();
    debug!(LOG, "{:?}", &transfer);
    if transfer.from.len() == 0 || !wallet::Wallet::validate_address(transfer.from.clone()) {
        return bad_data_json!("ERROR: From's address is not valid".to_owned());
    }
    if transfer.to.len() == 0 || !wallet::Wallet::validate_address(transfer.to.clone()) {
        return bad_data_json!("ERROR: To's address is not valid".to_owned());
    }
    if transfer.secret_key.len() == 0 {
        return bad_data_json!("ERROR: From's secret key is not valid".to_owned());
    }
    if transfer.amount <= 0 {
        return bad_data_json!("ERROR: amount must more than zero".to_owned());
    }
    let secret_key = util::decode_hex(&transfer.secret_key);
    let from_wallet = wallet::Wallet::recover_wallet(&secret_key);
    if from_wallet.is_err() {
        return bad_data_json!(from_wallet.err().unwrap());
    }
    let from_wallet = from_wallet.unwrap();
    if from_wallet.get_address() != transfer.from {
        return bad_data_json!("from's addr not equal secret_key's addr".to_owned());
    }

    let (to, amount) = (&transfer.to, transfer.amount as isize);
    let mem_pool = state.mem_pool.clone();
    let mem_pool = mem_pool.lock().unwrap();
    let spend_utxos = {
        // 1. get the input's reference transaction
        let pub_key = util::public_key_to_vec(&from_wallet.public_key, false);
        let mut spend_utxos = HashMap::new();
        for (txid, tx) in &*mem_pool {
            for vin in &tx.vin {
                if vin.uses_key(&pub_key) {
                    // it it from's unspend utxo
                    let ref_out_txid = util::encode_hex(&vin.txid);
                    let ref_out_idx = vin.vout;
                    spend_utxos.entry(ref_out_txid).or_insert(vec![]).push(
                        ref_out_idx,
                    );
                }
            }
        }
        spend_utxos
    };
    let bc = &state.bc.lock().unwrap();
    let tx = bc.create_new_utxo_transaction(&from_wallet, to, amount, Some(spend_utxos));
    if tx.is_err() {
        return bad_data_json!(tx.err().unwrap());
    }
    let tx = tx.unwrap();
    let local_addr = state.local_node.clone();
    let known_nodes = state.known_nodes.clone();
    let central_node = {
        let known_nodes = known_nodes.lock().unwrap();
        known_nodes[0].clone()
    };
    send_tx(known_nodes, &central_node, &local_addr, &tx);
    ok_json!()
}

#[post("/addr", format = "application/json", data = "<addrs>")]
pub fn handle_addr(state: rocket::State<router::BlockState>, addrs: Json<Addr>) -> Json<Value> {
    let local_node = state.local_node.clone();
    let addr_list = addrs.addr_list.clone();
    {
        let known_nodes_lock = state.known_nodes.clone();
        let mut known_nodes = known_nodes_lock.lock().unwrap();
        addr_list.into_iter().for_each(|addr| {
            let exist = known_nodes.clone().into_iter().all(|node| {
                debug!(LOG, "{} {}", &node, &local_node);
                node != addr
            });
            if exist {
                known_nodes.push(addr);
            }
        });
    }
    request_blocks(state.known_nodes.clone(), &local_node);
    ok_json!()
}

#[post("/get_height_block", format = "application/json", data = "<height_data>")]
pub fn handle_get_heigt_block_data(state: rocket::State<router::BlockState>, height_data: Json<HeightBlock>) -> Json<Value>{
    let bc = &state.bc.lock().unwrap();
    let local_node = state.local_node.clone();
    let res = bc.block_with_height(height_data.height);
    if let Some(block) = res {
        send_block(
            state.known_nodes.clone(),
            &height_data.add_from,
            &local_node,
            block,
        );
        return ok_json!();
    }
    bad_data_json!(format!("not foud the height:{} block", height_data.height))
}

// sync data
#[post("/get_data", format = "application/json", data = "<block_data>")]
pub fn handle_get_block_data(
    state: rocket::State<router::BlockState>,
    block_data: Json<GetData>,
) -> Json<Value> {

    info!(
        LOG,
        "get data, {}, {}",
        &block_data.data_type,
        &block_data.id
    );
    let get_type = &block_data.data_type;
    let bc = &state.bc.lock().unwrap();
    let local_node = state.local_node.clone();
    if get_type == "block" {
        let block_hash = &block_data.id;
        let block = bc.block(block_hash);
        if block.is_none() {
            return bad_json!();
        }
        send_block(
            state.known_nodes.clone(),
            &block_data.add_from,
            &local_node,
            block.unwrap(),
        );
    }
    if get_type == "tx" {
        let txid = &block_data.id;
        let tx = {
            let mem_pool = state.mem_pool.clone();
            let mem_pool = mem_pool.lock().unwrap();
            let tx = mem_pool.get(&txid.clone());
            if tx.is_none() {
                return bad_data_json!(format!("{} not found in {}", &txid, &local_node));
            }
            tx.unwrap().clone()
        };
        warn!(
            LOG,
            "find a txid {}, transfer it to {}",
            txid,
            &block_data.add_from
        );
        send_tx(
            state.known_nodes.clone(),
            &block_data.add_from,
            &local_node,
            &tx,
        );
    }

    ok_json!()
}

// Notic, it may be cause mining ...
#[post("/tx", format = "application/json", data = "<tx>")]
pub fn handle_tx(state: rocket::State<router::BlockState>, tx: Json<TX>) -> Json<Value> {
    let txdata = &tx.transaction;
    let ts: Transaction = serde_json::from_slice(txdata).unwrap();
    let txid = util::encode_hex(&ts.id);

    debug!(LOG, "get a transaction, txid: {}", &txid);
    // add new transaction into mempool
    let mut mem_pool = state.mem_pool.lock().unwrap();
    mem_pool.entry(txid).or_insert(ts.clone());
    // local node addr
    let local_node = state.local_node.clone();
    let bc = &state.bc.lock().unwrap();
    let known_nodes = state.known_nodes.clone();
    let ref known_nodes = {
        let know_nodes = known_nodes.lock().unwrap();
        know_nodes.clone()
    };
    // the local node is central node, it just do forward the new transactions to other nodes in the network.
    if local_node.to_lowercase() == known_nodes[0].to_lowercase() {
        let txid = &ts.id;
        for node in known_nodes {
            info!(LOG, "forward transaction to {}", &node);
            send_inv(
                state.known_nodes.clone(),
                node,
                &local_node,
                "tx",
                vec![txid.to_vec()],
            );
        }
    } else if state.mining_address.clone().len() > 0 {
        // the local node is a mining node, start to mining!
        let mining_addr = state.mining_address.clone();

        info!(LOG, "start to mining...");
        loop {
            if mem_pool.len() >= MINING_SIZE {
                debug!(LOG, "i am mining node:{} start to mining", &mining_addr);
                let mut mem_pool_copy = mem_pool.clone();
                let res = bc.mine_new_block(state.mining_address.to_string(), &mut mem_pool_copy);
                if let Err(ref e) = res {
                    error!(LOG, "mine block fail, err:{}", e);
                    return bad_data_json!(e);
                }
                let new_block_hash = res.unwrap();
                info!(
                    LOG,
                    "🔨 🔨 🔨 mining a new block, hash is {}",
                    &new_block_hash
                );
                *mem_pool = mem_pool_copy;

                // notify other nodes to sync the new block
                &known_nodes.into_iter().for_each(|node| if *node !=
                    local_node.to_string()
                {
                    send_inv(
                        state.known_nodes.clone(),
                        &node,
                        &local_node,
                        "block",
                        vec![util::decode_hex(&new_block_hash)],
                    )
                });
            }
            if mem_pool.len() <= MINING_SIZE {
                debug!(LOG, "stop mining ...");
                break;
            }
        }
    }
    ok_json!()
}

// sync block, return all block hashes
#[post("/get_blocks", format = "application/json", data = "<blocks>")]
pub fn handle_get_blocks(
    state: rocket::State<router::BlockState>,
    blocks: Json<GetBlocks>,
) -> Json<Value> {

    let bc = &state.bc.lock().unwrap();
    let ref hashes: Vec<String> = bc.block_hashes();
    let hashes_vec: Vec<Vec<u8>> = hashes
        .into_iter()
        .map(|item| util::decode_hex(&item))
        .collect();
    send_inv(
        state.known_nodes.clone(),
        &blocks.add_from,
        &state.local_node,
        "block",
        hashes_vec,
    );
    ok_data_json!(hashes)
}

#[post("/version", format = "application/json", data = "<version>")]
pub fn handle_version(
    state: rocket::State<router::BlockState>,
    version: Json<Version>,
) -> Json<Value> {
    let bc = &state.bc.lock().unwrap();
    let my_best_height = bc.best_height();
    let foreigner_best_height = version.best_height;
    let local_node = state.local_node.clone();
    if my_best_height < foreigner_best_height {
        send_get_block(state.known_nodes.clone(), &version.addr_from, &local_node);
    } else if my_best_height > foreigner_best_height {
        send_version(
            state.known_nodes.clone(),
            &version.addr_from,
            "/version",
            &local_node,
            bc.block_chain(),
        );
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
    let bc = &state.bc.lock().unwrap();

    let inv_type = &inv.inv_type;
    let add_from = &inv.add_from;
    let local_node = state.local_node.clone();
    if inv_type == "block" {
        let block_in_transit = state.block_in_transit.clone();
        let mut block_in_transit = block_in_transit.lock().unwrap();
        let last_height = bc.best_height() as usize;
        if last_height > (inv.items.len() - 1){
            return ok_json!();
        }

        inv.items.clone().into_iter().for_each(|item| {
            debug!(
                LOG,
                "addr_from:{}, block item:{}",
                add_from,
                util::encode_hex(item)
            );
        });

        let step = inv.items.len() - last_height - 1;
        if step <= 0 {
            return ok_json!();
        }
        let mut items = inv.items[0..step].to_vec();
        // reverse it
        items.reverse();
        let block_hash = items[0].clone();
        *block_in_transit = items;
        block_in_transit.clone().into_iter().for_each(|item| {
            println!("addr_from:{}, block item:{}", add_from, util::encode_hex(item));
        });

        send_get_data(
            state.known_nodes.clone(),
            add_from,
            &local_node,
            "block".to_owned(),
            block_hash.clone(),
        );

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
        let mem_pool = Arc::clone(&state.mem_pool);
        let mem_pool = mem_pool.lock().unwrap();
        if mem_pool.get(&util::encode_hex(&txid)).is_none() {
            info!(
                LOG,
                "{} transaction not found in local node, start to download from remote node:{}",
                util::encode_hex(&txid),
                &inv.add_from
            );
            send_get_data(
                state.known_nodes.clone(),
                add_from,
                &local_node,
                "tx".to_owned(),
                txid,
            );
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
    let bc = &state.bc.lock().unwrap();
    let local_node = state.local_node.clone();
    let central_node = {
        state.known_nodes.lock().unwrap()[0].to_string()
    };
    let new_block = block::Block::try_deserialize_block(&block_data.block);
    if new_block.is_err() {
        return bad_data_json!(new_block.err().unwrap());
    }
    let new_block = new_block.unwrap();
    let block_hash = new_block.hash.clone();
    let res = bc.add_new_block(&new_block, block_data.add_from == central_node);
    if let Err(e) = res {
        error!(LOG, "add block faild, err:{:?}", e);
        return bad_data_json!(e);
    }else if let Ok(true) = res {
        info!(LOG, "{} has exists, ignore it", util::encode_hex(block_hash));
        return ok_json!();
    }

    info!(
        LOG,
        "added block successfully, block source:{}, block hash: {} ",
        &block_data.add_from,
        util::encode_hex(&block_hash)
    );

    info!(
        LOG,
        "prepare to update utxos, the new block is {}",
        util::encode_hex(&block_hash)
    );
    bc.update_utxo(&new_block);
    info!(LOG, "update utxos successfully.");

    // TODO why do it in that.
    let block_in_transit = state.block_in_transit.clone();
    {
        let mut bc_in_transit = block_in_transit.lock().unwrap();
        if bc_in_transit.len() > 0 {
            let block_hash = bc_in_transit[0].clone();
            send_get_data(
                state.known_nodes.clone(),
                &block_data.add_from,
                &local_node,
                "block".to_owned(),
                block_hash,
            );
            *bc_in_transit = bc_in_transit[1..].to_vec();
        }
    }

    ok_data_json!("")
}


#[get("/")]
fn index() -> io::Result<NamedFile> {
    NamedFile::open("static/index.html")
}

// TODO, may be has better way to do it
fn request_blocks(know_nodes: Arc<Mutex<Vec<String>>>, local_node: &str) {
    let know_nodes_copy = {
        let know_nodes = know_nodes.clone();
        let know_nodes = know_nodes.lock().unwrap();
        know_nodes.clone().into_iter().map(|node| node.clone())
    };
    for node in know_nodes_copy {
        send_get_block(know_nodes.clone(), &node, local_node)
    }
}

// path => /get_data
fn send_get_data(
    known_nodes: Arc<Mutex<Vec<String>>>,
    addr: &str,
    local_node: &str,
    kind: String,
    id: Vec<u8>,
) {
    let data = GetData {
        add_from: local_node.to_owned(),
        data_type: kind,
        id: util::encode_hex(&id),
    };
    let data = serde_json::to_vec(&data).unwrap();
    do_post_request(known_nodes, addr, "/get_data", &data);
}

// path => /addr
pub fn send_addr(know_nodes: Arc<Mutex<Vec<String>>>, addr: &str, addr_list: Vec<String>) {
    let join_cluster = Addr { addr_list: addr_list };
    let data = serde_json::to_vec(&join_cluster).unwrap();
    do_post_request(know_nodes, addr, "/addr", &data);
}

// send local node version to remote addr
// path => /version
pub fn send_version(
    known_nodes: Arc<Mutex<Vec<String>>>,
    addr: &str,
    path: &str,
    local_node: &str,
    bc: Arc<BlockChain>,
) {
    let best_height = bc.get_best_height();
    let version = Version::new(NODE_VERSION, best_height, local_node.to_owned());
    let data = &serde_json::to_vec(&version).unwrap();
    do_post_request(known_nodes, addr, path, data);
}
// path => /tx
pub fn send_tx(
    known_nodes: Arc<Mutex<Vec<String>>>,
    addr: &str,
    local_node: &str,
    tx: &Transaction,
) {
    let data = serde_json::to_vec(&TX {
        add_from: local_node.to_owned(),
        transaction: serde_json::to_vec(tx).unwrap(),
    }).unwrap();
    do_post_request(known_nodes, addr, "/tx", &data);
}

// path => /block
fn send_block(
    known_nodes: Arc<Mutex<Vec<String>>>,
    addr: &str,
    local_node: &str,
    block: block::Block,
) {
    let data = serde_json::to_vec(&Block {
        add_from: local_node.to_owned(),
        block: serde_json::to_vec(&block).unwrap(),
    }).unwrap();
    do_post_request(known_nodes, addr, "/block", &data);
}

// path => /get_blocks
fn send_get_block(known_nodes: Arc<Mutex<Vec<String>>>, addr: &str, local_node: &str) {
    let request = GetBlocks { add_from: local_node.to_owned() };
    let data = serde_json::to_vec(&request).unwrap();
    do_post_request(known_nodes, addr, "/get_blocks", &data);
}

// path => /inv
fn send_inv(
    known_nodes: Arc<Mutex<Vec<String>>>,
    addr: &str,
    local_node: &str,
    kind: &str,
    items: Vec<Vec<u8>>,
) {
    let inventory = Inv {
        add_from: local_node.to_owned(),
        inv_type: kind.to_owned(),
        items: items,
    };
    let data = serde_json::to_vec(&inventory).unwrap();
    do_post_request(known_nodes, addr, "/inv", &data);
}

fn do_get_request(known_nodes: Arc<Mutex<Vec<String>>>, addr: &str, path: &str, data: &[u8]) {
    send_data(known_nodes, addr, path, "GET", vec![], data);
}

fn do_post_request(known_nodes: Arc<Mutex<Vec<String>>>, addr: &str, path: &str, data: &[u8]) {
    send_data(known_nodes, addr, path, "POST", vec![], data);
}

fn send_data(
    known_nodes: Arc<Mutex<Vec<String>>>,
    addr: &str,
    path: &str,
    method: &str,
    headers: Vec<(String, String)>,
    data: &[u8],
) {
    let arg = pool::DataArg::new(
        addr.to_owned(),
        path.to_owned(),
        method.to_owned(),
        headers,
        data,
    );
    pool::put_job(arg);
    {
        let mut known_nodes = known_nodes.lock().unwrap();
        let flag = known_nodes.clone().into_iter().all(|ref node| node != addr);
        if flag {
            known_nodes.append(&mut vec![addr.to_string()]);
        }
    }
}
