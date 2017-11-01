extern crate sapper;
extern crate sapper_body;
extern crate sapper_std;
extern crate serde_json;

use self::sapper::{Result as SapResult, Request, Response, SapperAppShell, SapperModule,
                   SapperRouter};
use self::sapper_std::{PathParams, FormParams, QueryParams, JsonParams};

use super::http_server;
use super::command::Addr;

#[derive(Clone)]
pub struct Node;

impl Node {
    fn index(req: &mut Request) -> SapResult<Response> {
        let mut resp = Response::new();
        resp.write_body("hello, boy!".to_string());
        Ok(resp)
    }

    fn add(req: &mut Request) -> SapResult<Response> {
        let addr: Addr = get_json_params!(req);
        res_json!(addr)
    }
}

impl SapperModule for Node {
    fn router(&self, router: &mut SapperRouter) -> SapResult<()> {
        router.get("/index", Node::index);
        Ok(())
    }
}
