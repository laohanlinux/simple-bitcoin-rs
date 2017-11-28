#!/bin/bash 

# tow central node
# tow wallet node 
# tow mining node

echo "start all node"

ps | grep bitcoin | awk '{print $1}' | while read line ; do kill $line ; done

echo "cear dirty files"

baseStore="blockchain_db"
base3000="3000_blockchain_db"
base3001="3001_blockchain_db"
base3010="3010_blockchain_db"
base3011="3011_blockchain_db"
base3020="3020_blockchain_db"
base3021="3021_blockchain_db"

rm -fr $baseStore $base3000 $base3001 $base3010 $base3011 $base3020 $base3021 

echo "create a genius blockchain_db"

RUST_BACKTRACE=full ./target/debug/bitcoin create_blockchain --address 17tQE4NbkiTroRwCeqEQF4Y9yVFBGLpL59 --store $baseStore 

cp -fr $baseStore $base3000 
cp -fr $baseStore $base3001 
cp -fr $baseStore $base3010
cp -fr $baseStore $base3011 
cp -fr $baseStore $base3020 
cp -fr $baseStore $base3021

echo "start central node "

RUST_BACKTRACE=full nohup ./target/debug/bitcoin server --addr 127.0.0.1 --port 3000 --central_node 127.0.0.1:3000 --store "3000_blockchain_db" --node_role central > 3000.log 2>&1 &

echo "start a mining node"

sleep 1
RUST_BACKTRACE=full nohup ./target/debug/bitcoin server --addr 127.0.0.1 --port 3020 --central_node 127.0.0.1:3000 --store "3020_blockchain_db" --node_role mining --mining_addr 17tQE4NbkiTroRwCeqEQF4Y9yVFBGLpL59 > 3020.log 2>&1 &

sleep 1
RUST_BACKTRACE=full nohup ./target/debug/bitcoin server --addr 127.0.0.1 --port 3021 --central_node 127.0.0.1:3000 --store "3021_blockchain_db" --node_role mining --mining_addr 16rBu48veHyj4AJeDTWE31x1n2D928uNfa > 3021.log 2>&1 &
