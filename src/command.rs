extern crate serde_json;

pub const NODE_VERSION: isize = 1;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Transfer {
    pub from: String,
    pub to: String,
    pub secret_key: String,
    pub amount: u32,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Addr {
    pub addr_list: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Block {
    pub add_from: String,
    pub block: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct GetBlocks {
    pub add_from: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct GetData {
    pub add_from: String,
    pub data_type: String,
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Inv {
    pub add_from: String,
    pub inv_type: String,
    pub items: Vec<Vec<u8>>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct TX {
    pub add_from: String,
    pub transaction: Vec<u8>,
}

// use to sync missing blocks
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Version {
    pub version: isize,
    pub best_height: isize,
    pub addr_from: String, // stores the address of the sender
}

impl Version {
    pub fn new(ver: isize, best_height: isize, addr_from: String) -> Version {
        Version {
            version: ver,
            best_height: best_height,
            addr_from: addr_from,
        }
    }
}
