extern crate rocket_contrib;

macro_rules! ok_data_json {
   ($data:expr) =>(
       Json(json!({"status":"ok", "data": $data}))
   )
}

macro_rules! ok_json {
    () => (Json(json!({"status": "ok"})))
}

macro_rules! bad_json {
    () => (Json(json!({"status": "bad"})))
}

macro_rules! bad_data_json {
    ($data:expr) => (
        Json(json!({"status": "fail", "msg": format!("{:?}", $data)}))
    )
}
