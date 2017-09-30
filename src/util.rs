extern crate hex;

pub fn encode_hex<T: AsRef<[u8]>>(data: T) -> String {
    hex::encode(data)
}

pub fn decode_hex<T: AsRef<[u8]>>(data: T) -> Vec<u8> {
    hex::decode(data).unwrap()
}
