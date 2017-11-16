extern crate threadpool;
extern crate tokio_core;
extern crate tokio_request;
extern crate url;

use self::tokio_core::reactor::Core;
use self::tokio_request::str::{get, post};
use self::threadpool::ThreadPool;

use std::sync::{Arc, Mutex, RwLock};
use std::io;
use std::io::Write;

use log::*;

lazy_static!{
    static ref POOL: Arc<Mutex<ThreadPool>> = Arc::new(Mutex::new(ThreadPool::new(10)));
//    static ref POOL: Arc<RwLock<ThreadPool>> = Arc::new(RwLock::new(ThreadPool::new(10)));
}

#[derive(Clone)]
pub struct DataArg {
    addr: String,
    path: String,
    method: String,
    headers: Vec<(String, String)>,
    data: Vec<u8>,
}

impl DataArg {
    pub fn new(
        addr: String,
        path: String,
        method: String,
        mut headers: Vec<(String, String)>,
        data: &[u8],
    ) -> DataArg {
        headers.push(("content-type".to_owned(), "application/json".to_owned()));
        DataArg {
            addr: addr,
            path: path,
            method: method,
            headers: headers,
            data: data.to_vec(),
        }
    }
}

pub fn put_job(data_arg: DataArg) {
    let pool = {
        let p = POOL.clone();
        let pool = p.lock().unwrap();
        pool.clone()
    };

    pool.execute(move || {
        let (addr, path, method, data, headers) = (
            &data_arg.addr,
            &data_arg.path,
            &data_arg.method,
            &data_arg.data,
            &data_arg.headers,
        );
        let addr = format!("http://{}{}", addr, path);
        debug!(LOG, "addr => {}", &addr);
        let mut evloop = Core::new().unwrap();

        let mut req = if method.as_str() == "GET" {
            get(&addr)
        } else {
            post(&addr)
        };
        let future = req.headers(headers.clone()).body(data.to_vec()).send(
            evloop.handle(),
        );
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
        writeln!(io::stdout(), "{}", String::from_utf8_lossy(body));
    });
}

#[cfg(test)]
mod tests {
    extern crate time;

    use super::DataArg;

    use std::thread;

    #[test]
    fn test_pool() {
        let timestamp = (
            "x-timestamp".to_owned(),
            format!("{}", time::get_time().sec),
        );
        let xt = (
            "x-t".to_owned(),
            format!("{}", "9bdbb499e4c9deed2c4a3e355ea2d580"),
        );
        let headers = vec![timestamp, xt];
        let arg = DataArg::new(
            "127.0.0.1:17172".to_owned(),
            "/1/users/state".to_owned(),
            headers,
            b"{\"uids\": [\"123\"]}",
        );
        super::put_job(arg.clone());
        super::put_job(arg.clone());
        super::put_job(arg.clone());
        super::put_job(arg.clone());
        super::put_job(arg.clone());
        super::put_job(arg.clone());
        super::put_job(arg.clone());
        super::put_job(arg.clone());
        super::put_job(arg.clone());
        super::put_job(arg.clone());
        super::put_job(arg.clone());
        super::put_job(arg.clone());
        super::put_job(arg.clone());
        super::put_job(arg.clone());
        super::put_job(arg.clone());
        super::put_job(arg.clone());
        super::put_job(arg.clone());
        super::put_job(arg.clone());
        super::put_job(arg.clone());
        super::put_job(arg.clone());
        super::put_job(arg.clone());

        thread::sleep_ms(2000);
    }
}
