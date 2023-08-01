// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use futures_util::{stream, StreamExt};
use hyper::header::{InvalidHeaderName, InvalidHeaderValue};
use hyper::Body;
use serde::{Deserialize, Serialize};
use serde_with::{self, serde_as};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::sync::mpsc;

/// A request sent by the websocket client.
#[derive(Debug, Deserialize)]
pub(crate) struct Event {
    #[serde(skip)]
    pub target: Target,
    pub topic: String,
    pub data: RequestData,
}

/// A request sent by the websocket client.
#[derive(Debug, Deserialize)]
pub(crate) struct MessageRequest {
    pub headers: serde_json::Map<String, serde_json::Value>,
    pub event: Event,
}

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub(crate) enum Target {
    #[default]
    None,
    Contract(String), // 0x01
    Host(String),     // 0x02
    Debugger(String), // 0x03
}

impl TryFrom<&str> for Target {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let paths: Vec<_> =
            value.split('/').skip_while(|p| p.is_empty()).collect();
        let target_type: i32 = paths
            .first()
            .ok_or_else(|| anyhow::anyhow!("Missing target type"))?
            .parse()?;
        let target = paths
            .get(1)
            .ok_or_else(|| anyhow::anyhow!("Missing target"))?
            .to_string();

        let target = match target_type {
            0x01 => Target::Contract(target),
            0x02 => Target::Host(target),
            0x03 => Target::Debugger(target),
            ty => {
                return Err(anyhow::anyhow!("Unsupported target type '{ty}'"))
            }
        };

        Ok(target)
    }
}

impl MessageRequest {
    pub fn x_headers(&self) -> serde_json::Map<String, serde_json::Value> {
        let mut h = self.headers.clone();
        h.retain(|k, _| k.to_lowercase().starts_with("x-"));
        h
    }

    pub fn header(&self, name: &str) -> Option<&serde_json::Value> {
        self.headers
            .iter()
            .find_map(|(k, v)| k.eq_ignore_ascii_case(name).then_some(v))
    }

    pub fn parse(bytes: &[u8]) -> anyhow::Result<Self> {
        let (headers, bytes) = parse_header(bytes)?;
        let event = Event::parse(bytes)?;
        Ok(Self { event, headers })
    }

    pub async fn from_request(
        req: hyper::Request<hyper::Body>,
    ) -> anyhow::Result<(Self, bool)> {
        let headers = req
            .headers()
            .iter()
            .filter_map(|(k, v)| {
                let a = v.as_bytes();
                serde_json::from_slice::<serde_json::Value>(a)
                    .ok()
                    .map(|v| (k.to_string(), v))
            })
            .collect();
        let (event, is_binary) = Event::from_request(req).await?;

        let req = MessageRequest { event, headers };

        Ok((req, is_binary))
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct MessageResponse {
    pub headers: serde_json::Map<String, serde_json::Value>,

    /// The data returned by the contract call.
    pub data: ResponseData,

    /// A possible error happening during the contract call.
    pub error: Option<String>,
}

impl MessageResponse {
    pub fn from_error(error: String) -> Self {
        Self {
            headers: serde_json::Map::default(),
            data: ResponseData::None,
            error: Some(error),
        }
    }

    pub fn into_http(
        self,
        is_binary: bool,
    ) -> anyhow::Result<hyper::Response<hyper::Body>> {
        if let Some(error) = &self.error {
            return Ok(hyper::Response::builder()
                .status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
                .body(hyper::Body::from(error.to_string()))?);
        }

        let body = {
            match self.data {
                ResponseData::Binary(wrapper) => {
                    let data = match is_binary {
                        true => wrapper.inner,
                        false => hex::encode(wrapper.inner).as_bytes().to_vec(),
                    };
                    Body::from(data)
                }
                ResponseData::Text(text) => Body::from(text),
                ResponseData::Channel(channel) => Body::wrap_stream(
                    stream::iter(channel).map(move |e| match is_binary {
                        true => Ok::<_, anyhow::Error>(e),
                        false => Ok::<_, anyhow::Error>(
                            hex::encode(e).as_bytes().to_vec(),
                        ),
                    }), // Ok::<_, anyhow::Error>),
                ),
                ResponseData::None => Body::empty(),
            }
        };

        Ok(hyper::Response::new(body))
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RequestData {
    Binary(BinaryWrapper),
    Text(String),
}

impl RequestData {
    pub fn as_bytes(&self) -> Vec<u8> {
        match self {
            Self::Binary(w) => w.inner.clone(),
            Self::Text(s) => s.as_bytes().to_vec(),
        }
    }
}

impl From<String> for RequestData {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}
impl From<Vec<u8>> for RequestData {
    fn from(value: Vec<u8>) -> Self {
        Self::Binary(BinaryWrapper { inner: value })
    }
}

/// Data in a response.
#[derive(Debug, Default)]
pub enum ResponseData {
    Binary(BinaryWrapper),
    Text(String),
    Channel(mpsc::Receiver<Vec<u8>>),
    #[default]
    None,
}

impl serde::Serialize for ResponseData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let str = match self {
            Self::Text(s) => s.to_string(),
            Self::Binary(w) => hex::encode(&w.inner),
            _ => String::default(),
        };
        serializer.serialize_str(&str)
    }
}

impl From<String> for ResponseData {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl From<Vec<u8>> for ResponseData {
    fn from(bytes: Vec<u8>) -> Self {
        Self::Binary(BinaryWrapper { inner: bytes })
    }
}

impl From<mpsc::Receiver<Vec<u8>>> for ResponseData {
    fn from(receiver: mpsc::Receiver<Vec<u8>>) -> Self {
        Self::Channel(receiver)
    }
}

#[serde_with::serde_as]
#[derive(Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct BinaryWrapper {
    #[serde_as(as = "serde_with::hex::Hex")]
    pub inner: Vec<u8>,
}

impl Event {
    pub fn parse(bytes: &[u8]) -> anyhow::Result<Self> {
        let (topic, bytes) = parse_string(bytes)?;
        let data = bytes.to_vec().into();

        Ok(Self {
            target: Target::None,
            topic,
            data,
        })
    }
    pub async fn from_request(
        req: hyper::Request<hyper::Body>,
    ) -> anyhow::Result<(Self, bool)> {
        let (parts, req_body) = req.into_parts();
        // HTTP REQUEST
        let is_binary = parts
            .headers
            .get(CONTENT_TYPE)
            .and_then(|h| {
                h.to_str().ok().map(|s| s.starts_with(CONTENT_TYPE_BINARY))
            })
            .unwrap_or_default();

        let target = parts.uri.path().try_into()?;

        let body = hyper::body::to_bytes(req_body).await?;

        let mut event = match is_binary {
            true => Event::parse(&body)
                .map_err(|e| anyhow::anyhow!("Invalid data {e}"))?,
            false => serde_json::from_slice(&body)
                .map_err(|e| anyhow::anyhow!("Invalid data {e}"))?,
        };
        event.target = target;
        Ok((event, is_binary))
    }
}
const CONTENT_TYPE: &str = "Content-Type";
const CONTENT_TYPE_BINARY: &str = "application/octet-stream";

fn parse_len(bytes: &[u8]) -> anyhow::Result<(usize, &[u8])> {
    if bytes.len() < 4 {
        return Err(anyhow::anyhow!("not enough bytes"));
    }

    let len =
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    let (_, left) = bytes.split_at(len);

    Ok((len, left))
}

type Header<'a> = (serde_json::Map<String, serde_json::Value>, &'a [u8]);
fn parse_header(bytes: &[u8]) -> anyhow::Result<Header> {
    let (len, bytes) = parse_len(bytes)?;
    if bytes.len() < len {
        return Err(anyhow::anyhow!("not enough bytes for parsed len {len}"));
    }

    let (header_bytes, bytes) = bytes.split_at(len);
    let header = serde_json::from_slice(header_bytes)?;

    Ok((header, bytes))
}

fn parse_target_type(bytes: &[u8]) -> anyhow::Result<(u8, &[u8])> {
    if bytes.is_empty() {
        return Err(anyhow::anyhow!("not enough bytes for target type"));
    }

    let (target_type_bytes, bytes) = bytes.split_at(1);
    let target_type = target_type_bytes[0];

    Ok((target_type, bytes))
}

fn parse_string(bytes: &[u8]) -> anyhow::Result<(String, &[u8])> {
    let (len, bytes) = parse_len(bytes)?;
    if bytes.len() < len {
        return Err(anyhow::anyhow!("not enough bytes for parsed len {len}"));
    }

    let (string_bytes, bytes) = bytes.split_at(len);
    let string = String::from_utf8(string_bytes.to_vec())?;

    Ok((string, bytes))
}

#[derive(Debug)]
pub enum ExecutionError {
    Http(hyper::http::Error),
    Hyper(hyper::Error),
    Json(serde_json::Error),
    Protocol(tungstenite::error::ProtocolError),
    Tungstenite(tungstenite::Error),
    Generic(anyhow::Error),
}

impl Display for ExecutionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionError::Http(err) => write!(f, "{err}"),
            ExecutionError::Hyper(err) => write!(f, "{err}"),
            ExecutionError::Json(err) => write!(f, "{err}"),
            ExecutionError::Protocol(err) => write!(f, "{err}"),
            ExecutionError::Tungstenite(err) => write!(f, "{err}"),
            ExecutionError::Generic(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for ExecutionError {}

impl From<anyhow::Error> for ExecutionError {
    fn from(err: anyhow::Error) -> Self {
        Self::Generic(err)
    }
}

impl From<hyper::http::Error> for ExecutionError {
    fn from(err: hyper::http::Error) -> Self {
        Self::Http(err)
    }
}

impl From<hyper::Error> for ExecutionError {
    fn from(err: hyper::Error) -> Self {
        Self::Hyper(err)
    }
}

impl From<serde_json::Error> for ExecutionError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}

impl From<InvalidHeaderName> for ExecutionError {
    fn from(value: InvalidHeaderName) -> Self {
        Self::Generic(value.into())
    }
}

impl From<InvalidHeaderValue> for ExecutionError {
    fn from(value: InvalidHeaderValue) -> Self {
        Self::Generic(value.into())
    }
}

impl From<tungstenite::error::ProtocolError> for ExecutionError {
    fn from(err: tungstenite::error::ProtocolError) -> Self {
        Self::Protocol(err)
    }
}

impl From<tungstenite::Error> for ExecutionError {
    fn from(err: tungstenite::Error) -> Self {
        Self::Tungstenite(err)
    }
}
