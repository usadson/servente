// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use crate::handler::HandlerController;

use crate::http::message::{
    Request,
    Response, StatusCode,
};

fn ping_test(request: &Request) -> Result<Response, anyhow::Error> {
    println!("[TEST] Ping: {:#?}", request);
    Ok(Response::with_status_and_string_body(StatusCode::Ok, ""))
}

pub fn register(controller: &mut HandlerController) {
    controller.register_get("/test-ping".into(), &ping_test);
    controller.register_post("/test-ping".into(), &ping_test);
}
