// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use std::{
    collections::HashMap
};

use crate::http::message::{
    Request,
    Response, Method, StatusCode,
};

type HandlerReturnType = Result<Response, anyhow::Error>;

pub type Handler = &'static dyn Fn(&Request) -> HandlerReturnType;

#[derive(Clone)]
struct HandlerInfo {
    handler: Handler,
}

#[derive(Clone, Default)]
pub struct HandlerController {
    handlers: HashMap<String, HashMap<Method, HandlerInfo>>,
}

unsafe impl Send for HandlerController {}

impl HandlerController {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    #[must_use]
    pub fn check_handle(&self, request: &Request) -> Option<Result<Response, anyhow::Error>> {
        if let Some(handler_info) = self.handlers.get(request.target.as_str()) {
            let handled = handler_info.get(&request.method).map(|handler_info| {
                // TODO use block_on or something alike to avoid blocking the whole thread/task.
                (handler_info.handler)(request)
            });

            match handled {
                Some(response) => Some(response),
                None => Some(Ok(Response::with_status_and_string_body(StatusCode::MethodNotAllowed, "Method Not Allowed"))),
            }
        } else {
            None
        }
    }

    pub fn register(&mut self, path: String, handler: Handler, allowed_method: Method) {
        let info = HandlerInfo {
            handler,
        };
        match self.handlers.entry(path) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                entry.get_mut().insert(allowed_method, info);
            },
            std::collections::hash_map::Entry::Vacant(entry) => {
                let mut map = HashMap::new();
                map.insert(allowed_method, info);
                entry.insert(map);
            },
        }
    }

    // TODO support async
    pub fn register_get(&mut self, path: String, handler: Handler) {
        self.register(path, handler, Method::Get);
    }

    pub fn register_post(&mut self, path: String, handler: Handler) {
        self.register(path, handler, Method::Post);
    }
}
