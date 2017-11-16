#!/bin/bash 

echo "start all node"

ps | grep bitcoin | awk '{print $1}' | while read line ; do kill $line ; done

echo "cear dirty files"

baseStore="blockchain_db"
base3000="3000_blockchain_db"
base3001="3001_blockchain_db"
base3002="3002_blockchain_db"

rm -fr $baseStore $base3000 $base3001 $base3002  

echo "create a genius blockchain_db"

RUST_BACKTRACE=full ./target/debug/bitcoin create_blockchain --address 17tQE4NbkiTroRwCeqEQF4Y9yVFBGLpL59 --store $baseStore 

cp -fr $baseStore $base3000 
cp -fr $baseStore $base3001 
cp -fr $baseStore $base3002

echo "start central node "

RUST_BACKTRACE=full ./target/debug/bitcoin server --addr 127.0.0.1 --port 3000 --central_node 127.0.0.1:3000 --store "3000_blockchain_db" --node_role central &
sleep 1

echo "start a mining node"

RUST_BACKTRACE=full ./target/debug/bitcoin server --addr 127.0.0.1 --port 3001 --central_node 127.0.0.1:3000 --store "3001_blockchain_db" --node_role wallet &

sleep 1

echo "start a wallet node"

RUST_BACKTRACE=full ./target/debug/bitcoin server --addr 127.0.0.1 --port 3002 --central_node 127.0.0.1:3000 --store "3002_blockchain_db" --node_role mining &
