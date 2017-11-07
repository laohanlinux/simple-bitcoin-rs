extern crate rocket_contrib;

use self::rocket_contrib::{Json, Value};

macro_rules! ok_data_json{
   ($data:expr) =>(
       Json(json!({"status":"ok", "data": $data}))
   )
}

macro_rules! ok_json {
    () => (Json(json!({"status": "ok"})))
}

macro_rules! bad_json{
    () => (Json(json!({"status": "bad"})))
}
