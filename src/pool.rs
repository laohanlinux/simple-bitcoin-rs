extern crate threadpool;
extern crate tokio_core;
extern crate tokio_request;
extern crate url;

use self::tokio_core::reactor::Core;
use self::tokio_request::str::post;
use self::threadpool::ThreadPool;

use std::sync::{Arc, Mutex, RwLock};

use log::*;

lazy_static!{
    static ref POOL: Arc<Mutex<ThreadPool>> = Arc::new(Mutex::new(ThreadPool::new(10)));
//    static ref POOL: Arc<RwLock<ThreadPool>> = Arc::new(RwLock::new(ThreadPool::new(10)));
}

pub struct DataArg {
    addr: String,
    path: String,
    data: Vec<u8>,
}

pub fn PutJob(data_arg: DataArg) {
    let pool = {
        let p = POOL.clone();
        let pool = p.lock().unwrap();
        pool.clone()
    };

    pool.execute(move || {
        /*debug!(
        LOG,
        "address {}, path {}, data {}",
        addr,
        path,
        String::from_utf8_lossy(data)
    );*/
        let (addr, path, data) = (&data_arg.addr, &data_arg.path, &data_arg.data);
        let addr = format!("http://{}{}", addr, path);
        let mut evloop = Core::new().unwrap();
        let future = post(&addr)
            .header("content-type", "application/json")
            .body(data.to_vec())
            .send(evloop.handle());
        let result = evloop.run(future).expect("HTTP Request failed!");
        if result.is_success() == false {
            error!(
                LOG,
                "send get data fail, URI => {}",
                addr,
            );
        }
        let body = result.body();
        info!(
            LOG,
            "send get data successfully, URI => {}, data => {}",
            addr,
            String::from_utf8_lossy(body)
        );
    });
}
