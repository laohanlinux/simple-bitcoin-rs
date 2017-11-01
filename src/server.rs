extern crate serde_json;
extern crate rocket;
extern crate rocket_contrib;

use self::rocket_contrib::{Json, Value};
use self::rocket::State;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

type ID = usize;

#[derive(Serialize, Deserialize)]
struct Message {
    id: Option<ID>,
    contents: String,
}

#[post("/<id>", format = "application/json", data = "<message>")]
fn new(id: ID, message: Json<Message>, map: State<Message>) -> Json<Value>{
    Json(json!({"status": "ok"}))
}