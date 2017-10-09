use super::util;

#[derive(Debug, Clone)]
pub struct MerkleTree {
    pub root: Option<Box<MerkleNode>>,
}

#[derive(Debug, Clone)]
struct MerkleNode {
    data: Box<Vec<u8>>,
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
            let hash = util::sha256(&dataum);
            nodes.push(MerkleNode {
                data: Box::new(hash),
                left: None,
                right: None,
            });
        }

        let data_size = data.len();
        for _ in (0..data_size) {
            let mut new_level = vec![];
            let mut j = 0;
            while j < &nodes.len() / 2 {
                let node =
                    MerkleNode::new_merkle_node(nodes[j].clone(), nodes[j + 1].clone(), vec![]);
                new_level.push(node);
            }
            nodes = new_level;
        }

        MerkleTree { root: Some(Box::new(nodes.pop().unwrap())) }
    }
}

impl MerkleNode {
    fn new_merkle_node(left: MerkleNode, right: MerkleNode, data: Vec<u8>) -> MerkleNode {
        let mut merkle_tree_node = MerkleNode {
            data: Box::new(vec![]),
            left: None,
            right: None,
        };

        let mut hash_data = Vec::with_capacity(left.data.len() + right.data.len());
        hash_data[..left.data.len()].clone_from_slice(&left.data);
        hash_data[left.data.len()..].clone_from_slice(&right.data);

        let hash = util::sha256(&hash_data);
        merkle_tree_node.data = Box::new(hash);
        merkle_tree_node
    }
}
