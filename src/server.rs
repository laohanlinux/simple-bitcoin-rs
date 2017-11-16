extern crate serde_json;
extern crate rocket;
extern crate rocket_contrib;
extern crate lazy_static;

use self::rocket_contrib::{Json, Value};

use std::sync::{Arc, Mutex};

use transaction::Transaction;
use log::*;
use blockchain::BlockChain;
use command::*;
use router;
use util;
use block;
use utxo_set;
use wallet;
use pool;

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
    let (to, amount) = (&transfer.to, transfer.amount as isize);
    let bc = state.bc.clone();
    let utxo = utxo_set::UTXOSet::new(bc.clone());
    let tx = Transaction::new_utxo_transaction(&from_wallet, to.to_owned(), amount, &utxo).unwrap();
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
    {
        let known_nodes_lock = state.known_nodes.clone();
        let mut known_nodes = known_nodes_lock.lock().unwrap();
        let exist = known_nodes.clone().into_iter().all(|node| node != local_node.to_string());
        if !exist {
            known_nodes.append(&mut addrs.into_inner().addr_list);
        } 
        info!(LOG, "There are {} known nodes now", known_nodes.len());
    }
    request_blocks(state.known_nodes.clone(), &local_node);
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
    let local_node = state.local_node.clone();
    if get_type == "block" {
        let block = bc.get_block(&util::decode_hex(&block_data.id));
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
        let txid = util::encode_hex(&block_data.id);
        let tx = {
            let mem_pool = state.mem_pool.clone();
            let mem_pool = mem_pool.lock().unwrap();
            let tx = mem_pool.get(&txid);
            if tx.is_none() {
                return ok_json!();
            }
            tx.unwrap().clone()
        };
        send_tx(
            state.known_nodes.clone(),
            &block_data.add_from,
            &local_node,
            &tx,
        );
    }

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

    let known_nodes = state.known_nodes.clone();
    let ref known_nodes = {
        let know_nodes = known_nodes.lock().unwrap();
        know_nodes.clone()
    };
    // the local node is central node, it just do forward the new transactions to other nodes in the network.
    if local_node.to_lowercase() == known_nodes[0].to_lowercase() {
        let txid = &ts.id;
        for node in known_nodes {
            send_inv(
                state.known_nodes.clone(),
                node,
                &local_node,
                "tx",
                vec![txid.to_vec()],
            );
        }
    } else {
        // the local node is a mining node, start to mining!
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

                // notify other nodes to sync the new block
                &known_nodes.into_iter().for_each(|node| if *node !=
                    local_node.to_string()
                {
                    send_inv(
                        state.known_nodes.clone(),
                        &node,
                        &local_node,
                        "block",
                        vec![new_block.hash.clone()],
                    )
                });
            }
            if mem_pool.len() <= 0 {
                break;
            }
        }
    }
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
    let local_node = state.local_node.clone();
    if my_best_height < foreigner_best_height {
        send_get_block(state.known_nodes.clone(), &version.addr_from, &local_node);
    } else if my_best_height > foreigner_best_height {
        send_version(
            state.known_nodes.clone(),
            &version.addr_from,
            "/version",
            &local_node,
            bc,
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
    let inv_type = &inv.inv_type;
    if inv_type == "block" {
        let block_in_transit = state.block_in_transit.clone();
        let mut block_in_transit = block_in_transit.lock().unwrap();
        *block_in_transit = inv.items.clone();
        let block_hash = inv.items[0].clone();
        send_get_data(
            state.known_nodes.clone(),
            &inv.add_from,
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
        let mem_pool = state.mem_pool.clone();
        let mem_pool = mem_pool.lock().unwrap();
        if mem_pool.get(&util::encode_hex(&txid)).is_none() {
            send_get_data(
                state.known_nodes.clone(),
                &inv.add_from,
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
            send_get_data(
                state.known_nodes.clone(),
                &block_data.add_from,
                "block".to_owned(),
                block_hash,
            );
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

fn send_get_data(known_nodes: Arc<Mutex<Vec<String>>>, addr: &str, kind: String, id: Vec<u8>) {
    let data = GetData {
        add_from: addr.to_string(),
        data_type: kind,
        id: util::encode_hex(&id),
    };
    let data = serde_json::to_vec(&data).unwrap();
    let path = format!("{}/get_data", addr);
    send_data(known_nodes, addr, &path, &data);
}

// path 
pub fn send_addr(know_nodes: Arc<Mutex<Vec<String>>>,
    addr: &str,
    addr_list: Vec<String>){
    let join_cluster = Addr{
        addr_list: addr_list,
    };
    let data = serde_json::to_vec(&join_cluster).unwrap();
    send_data(know_nodes, addr, "/addr", &data);
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
    send_data(known_nodes, addr, "/inv", &data);
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
    send_data(known_nodes, addr, "/tx", &data);
}

// path => /block
fn send_block(
    known_nodes: Arc<Mutex<Vec<String>>>,
    addr: &str,
    local_node: &str,
    block: block::Block,
) {
    let data = serde_json::to_vec(&Block {
        add_from: addr.to_owned(),
        block: serde_json::to_vec(&block).unwrap(),
    }).unwrap();
    send_data(known_nodes, local_node, "/block", &data);
}

// path => /get_blocks
fn send_get_block(known_nodes: Arc<Mutex<Vec<String>>>, addr: &str, local_node: &str) {
    let request = GetBlock { add_from: local_node.to_owned() };
    let data = serde_json::to_vec(&request).unwrap();
    let path = format!("{}/get_blocks", addr);
    send_data(known_nodes, addr, &path, &data);
}

// send local node version to remote addr
// path => /version
fn send_version(
    known_nodes: Arc<Mutex<Vec<String>>>,
    addr: &str,
    path: &str,
    local_node: &str,
    bc: Arc<BlockChain>,
) {
    let best_height = bc.get_best_height();
    let version = Version::new(NODE_VERSION, best_height, local_node.to_owned());
    let data = &serde_json::to_vec(&version).unwrap();
    send_data(known_nodes, addr, path, data);
}

fn send_data(
    known_nodes: Arc<Mutex<Vec<String>>>,
    addr: &str,
    path: &str,
    data: &[u8],
){
    let arg = pool::DataArg::new(addr.to_owned(), path.to_owned(), vec![], data);
    pool::put_job(arg);
    // update known_nodes
    {
        let mut known_nodes = known_nodes.lock().unwrap();
        let flag = known_nodes.clone().into_iter().all(
            |ref node| node != addr,
        );
        if flag {
            known_nodes.append(&mut vec![addr.to_string()]);
        }
    }
}


