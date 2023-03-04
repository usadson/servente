// Copyright (C) 2023 Tristan Gerritsen <tristan@thewoosh.org>
// All Rights Reserved.

use anyhow::bail;

use crate::http::message::{
    Request,
    Method,
    HeaderName,
};

pub async fn parse_request() -> anyhow::Result<Request> {
    bail!("Not implemented");
}
