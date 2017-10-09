extern crate bigint;
extern crate lazy_static;

use self::bigint::U256;

lazy_static!{
    static ref MAX_NONCE: U256 = 10.into();
}
