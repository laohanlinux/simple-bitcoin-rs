extern crate serde_json;
extern crate validator;

use self::validator::{Validate, ValidationError};

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
    pub id: Vec<u8>,
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
pub struct Verzion {
    pub version: isize,
    pub best_hight: isize,
    pub addr_from: String, // stores the address of the sender
}
