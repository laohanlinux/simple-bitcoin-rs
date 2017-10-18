#!/bin/bash

address="17tQE4NbkiTroRwCeqEQF4Y9yVFBGLpL59";

case $1 in 
    "open")
        cargo build && RUST_BACKTRACE=1 ./target/debug/bitcoin open
    ;;
    "create_blockchain" )
        rm -fr /tmp/block_chain && mkdir /tmp/block_chain && cargo build && RUST_BACKTRACE=1 ./target/debug/bitcoin create_blockchain --address=17tQE4NbkiTroRwCeqEQF4Y9yVFBGLpL59
        ;;
    
    "balance" )
        cargo build && RUST_BACKTRACE=1 ./target/debug/bitcoin balance --address=$address
    ;;

    "print" )
        cargo build && RUST_BACKTRACE=1 ./target/debug/bitcoin print 
    ;;
    
    "reindex")
        cargo build && RUST_BACKTRACE=1 ./target/debug/bitcoin reindex 
    ;;

    "send")
        cargo build && RUST_BACKTRACE=1 ./target/debug/bitcoin send --mine=true --amount=1 --from="17tQE4NbkiTroRwCeqEQF4Y9yVFBGLpL59" --to="13vAhPZuRq2tsMb8t53DC3a6EcyD8GXahd" 
        ;;
esac
