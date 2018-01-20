extern crate chan;
extern crate bigint;

use std::thread;
use std::time::Duration;

use chan::{Sender, Receiver, tick_ms};
use bigint::U256;

use super::proof_of_work::ProofOfWork;
use super::util;
use super::block::Block;

const INTERNAL_MINE_TICK: u32 = 100;

/// return a sender channel for stop the work
pub fn async_mine(max_nonce: isize, proof_block: ProofOfWork, mine_recv: Sender<(isize, Vec<u8>, Block)>) -> Sender<()> {
    let (send, recv) = chan::sync(1);
    let b = proof_block.block.clone();
    let target = proof_block.target;
    thread::spawn(move||{
        let mut nonce: isize = -1;
        let b = b.clone();
        let proof_block = ProofOfWork{block: &b.clone(), target: target};
        assert_eq!(nonce, max_nonce);
        loop {
            nonce+=1;
            chan_select! {
                default => {
                    // start to mine
                    let data = proof_block.prepare_data(nonce);
                    let hash = util::sha256(&data);
                    let hash_int: U256 = util::as_u256(&hash);
                    if hash_int < proof_block.target {
                        mine_recv.send((nonce, hash, b.clone()));
                    }
                },
                recv.recv() => {
                    // receive stop mine
                    return;
                },
            };
        }
    });
    return send;
}