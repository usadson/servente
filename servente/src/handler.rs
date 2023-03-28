// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use std::{
    collections::HashMap
};

use servente_http::{
    HeaderName,
    Method,
    Request,
    Response,
    StatusCode,
};

use itertools::Itertools;

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
unsafe impl Sync for HandlerController {}

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

    /// If the request is referencing a resource which is a handler, returns the
    /// capabilities of that request. Otherwise, returns `None`.
    pub fn check_handle_options(&self, request: &Request) -> Option<Response> {
        let Some(handler_info) = self.handlers.get(request.target.as_str()) else {
            return None;
        };

        let mut response = Response::with_status(StatusCode::Ok);
        response.headers.set_content_length(0);

        #[allow(unstable_name_collisions)]
        let allowed_methods: String = handler_info.keys().map(|header| header.as_string())
            .intersperse(", ")
            .collect();

        response.headers.set(HeaderName::Allow, allowed_methods.clone().into());
        response.headers.set(HeaderName::AccessControlAllowMethods, allowed_methods.into());

        // TODO list the allowed origin here.
        response.headers.set(HeaderName::AccessControlAllowOrigin, "localhost:8080".into());

        // Read more at [resourcepolicy.fyi](https://resourcepolicy.fyi/)
        response.headers.set(HeaderName::CrossOriginResourcePolicy, "same-site".into());
        response.headers.set(HeaderName::Vary, "Origin".into());

        Some(response)
    }

    // TODO support async
    pub fn register_get(&mut self, path: String, handler: Handler) {
        self.register(path, handler, Method::Get);
    }

    pub fn register_post(&mut self, path: String, handler: Handler) {
        self.register(path, handler, Method::Post);
    }
}
