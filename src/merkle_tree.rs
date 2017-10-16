use super::util;

use std::iter;

#[derive(Debug, Clone)]
pub struct MerkleTree {
    pub root: Option<Box<MerkleNode>>,
}

#[derive(Debug, Clone)]
pub struct MerkleNode {
    pub data: Box<Vec<u8>>,
    left: Option<Box<MerkleNode>>,
    right: Option<Box<MerkleNode>>,
}

impl MerkleTree {
    // just build the root
    pub fn new_merkle_tree(mut data: Vec<Vec<u8>>) -> MerkleTree {
        // Fuck
        let clone_data = {
            if data.len() % 2 != 0 {
                let clone_data = {
                    data.last().unwrap()
                };
                Some(clone_data.clone())
            } else {
                None
            }
        };
        if clone_data.is_some() {
            data.push(clone_data.unwrap());
        }

        let mut nodes = vec![];
        for dataum in &data {
            nodes.push(MerkleNode::new(&dataum));
        }

        /**
         *
         *  |n1|n2|n3|n4|n5|n6
         **/
        let data_size = data.len();
        while true {
            let mut new_level = vec![];
            let (mut i, mut j) = (0, 0);
            while i < &nodes.len() / 2 {
                let node = MerkleNode::new_merkle_node(nodes[j].clone(), nodes[j + 1].clone());
                new_level.push(node);
                j += 2;
                i += 1;
            }
            nodes = new_level;
            if nodes.len() == 1 {
                break;
            }
        }
        MerkleTree { root: Some(Box::new(nodes.pop().unwrap())) }
    }
}

impl MerkleNode {
    fn new(data: &[u8]) -> MerkleNode {
        let mut mn: MerkleNode = Default::default();
        mn.data = Box::new(util::sha256(data));
        mn
    }
    fn new_merkle_node(left: MerkleNode, right: MerkleNode) -> MerkleNode {
        let mut merkle_tree_node: MerkleNode = Default::default();
        let mut hash_data = Vec::with_capacity(left.data.len() + right.data.len());
        hash_data.extend(iter::repeat(0).take(left.data.len() + right.data.len()));
        hash_data[..left.data.len()].clone_from_slice(&left.data);
        hash_data[left.data.len()..].clone_from_slice(&right.data);

        let hash = util::sha256(&hash_data);
        merkle_tree_node.data = Box::new(hash);
        merkle_tree_node
    }
}

impl Default for MerkleNode {
    fn default() -> MerkleNode {
        MerkleNode {
            data: Box::new(vec![]),
            left: None,
            right: None,
        }
    }
}
