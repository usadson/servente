// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use crate::{
    BodyKind,
    HeaderMap,
    HttpVersion,
    Method,
    RequestTarget,
};

#[derive(Debug)]
pub struct Request {
    pub method: Method,
    pub target: RequestTarget,
    pub version: HttpVersion,
    pub headers: HeaderMap,
    pub body: Option<BodyKind>,
}
