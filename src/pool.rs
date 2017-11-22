extern crate threadpool;
extern crate tokio_core;
extern crate tokio_request;
extern crate url;
extern crate serde_json;

use self::tokio_core::reactor::Core;
use self::tokio_request::str::{get, post};
use self::threadpool::ThreadPool;

use std::sync::{Arc, Mutex};
use std::io;
use std::io::Write;
use std::ops::Fn;

use log::*;
use util;

lazy_static!{
    static ref POOL: Arc<Mutex<ThreadPool>> = Arc::new(Mutex::new(ThreadPool::new(10)));
//    static ref POOL: Arc<RwLock<ThreadPool>> = Arc::new(RwLock::new(ThreadPool::new(10)));
}

pub struct DataArg {
    addr: String,
    path: String,
    method: String,
    headers: Vec<(String, String)>,
    data: Vec<u8>,
    call_back: Box<Fn(Vec<u8>) + Send>,
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
            call_back: default_call_back(),
        }
    }

    pub fn set_call_back(&mut self, call_back: Box<Fn(Vec<u8>) + Send>) {
        self.call_back = call_back;
    }
}

pub fn default_call_back() -> Box<Fn(Vec<u8>) + Send> {
    Box::new(|data| {
        let res: Data = match serde_json::from_slice(&data) {
            Ok(data) => data,
            Err(e) => {error!(LOG, "data:'{}' deserialize fail, err: {}", String::from_utf8_lossy(&data), e); return}
        };
        if res.status != "ok" {
            let data = String::from_utf8_lossy(&data);
            error!(LOG, "http request error: {}", data);
        }
    })
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct Data {
//    data: Vec<String>,
    status: String,
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
        //debug!(LOG, "addr => {}", &addr);
        let mut evloop = Core::new().unwrap();

        let req = if method.as_str() == "GET" {
            get(&addr)
        } else {
            post(&addr)
        };
        let future = req.headers(headers.clone()).body(data.to_vec()).send(
            evloop.handle(),
        );
        let result = evloop.run(future);
        if result.is_err(){
            error!(LOG, "{:?}", result.err());
            return;    
        }
        let result = result.unwrap();
        if result.is_success() {
            let body = result.body();
            (data_arg.call_back)(body.to_vec());
        } else {
            error!(
                LOG,
                "send get data fail, URI => {}",
                addr,
                );
        }
        /* let body = result.body();
           info!(
            LOG,
            "send get data successfully, URI => {}, data => {}",
            addr,
            String::from_utf8_lossy(body)
        );*/
        //writeln!(io::stdout(), "{}", String::from_utf8_lossy(body));
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
            "GET".to_owned(),
            headers,
            b"{\"uids\": [\"123\"]}",
        );
        /*        super::put_job(arg.clone());
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
*/
        thread::sleep_ms(2000);
    }
}
