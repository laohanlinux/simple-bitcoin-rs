extern crate serde_json;
extern crate validator;

use self::validator::{Validate, ValidationError};

pub const PROTOCOL: &str = "http";
pub const NODE_VERSION: isize = 1;

#[derive(Debug, Validate, Deserialize)]
struct SignupData {
    #[validate(email)]
    mail: String,
    #[validate(url)]
    site: String,
    #[serde(rename = "firstName")]
    first_name: String,
    #[validate(range(min = "18", max = "20"))]
    age: u32,
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

#[derive(Serialize, Deserialize, Debug, Validate, Default, Clone)]
pub struct GetBlock {
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
        Version{
            version: ver,
            best_height: best_height,
            addr_from: addr_from,
        } 
    }    
}
