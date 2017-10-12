extern crate shio;

use self::shio::prelude::*;

#[derive(Default)]
struct BlockHandler {}

impl shio::Handler for BlockHandler {
    type Result = Response;
    fn call(&self, _: Context) -> Self::Result {
        Response::with(format!("Hi, #{} (from thread: {:?}) \n", 0, 100))
    }
}
