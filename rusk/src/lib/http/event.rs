// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use hyper::header::{InvalidHeaderName, InvalidHeaderValue};
use serde::{Deserialize, Serialize};
use serde_with::{self, serde_as};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

/// A request sent by the websocket client.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Request {
    pub headers: serde_json::Map<String, serde_json::Value>,
    pub target: Target,
    pub topic: String,
    pub data: DataType,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) enum Target {
    Contract(String), // 0x01
    Host(String),     // 0x02
    Debugger(String), // 0x03
}

impl Request {
    pub fn x_headers(&self) -> serde_json::Map<String, serde_json::Value> {
        let mut h = self.headers.clone();
        h.retain(|k, _| k.to_lowercase().starts_with("x-"));
        h
    }
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub(crate) struct Response {
    pub headers: serde_json::Map<String, serde_json::Value>,

    /// The data returned by the contract call.
    pub data: DataType,

    /// A possible error happening during the contract call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Response {
    pub fn from_error(error: String) -> Self {
        Self {
            error: Some(error),
            ..Default::default()
        }
    }

    pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        Ok(vec![])
    }

    pub fn to_http(
        &self,
        binary: bool,
    ) -> anyhow::Result<hyper::Response<hyper::Body>> {
        let body = match binary {
            true => self.to_bytes(),
            false => serde_json::to_vec(&self.data)
                .map_err(|e| anyhow::anyhow!("Cannot ser {e}")),
        }?;
        Ok(hyper::Response::new(hyper::Body::from(body)))
    }
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(untagged)]
pub enum DataType {
    Binary(BinaryWrapper),
    #[default]
    None,
    Text(String),
}

impl DataType {
    pub fn as_bytes(&self) -> Vec<u8> {
        match self {
            Self::Binary(w) => w.inner.clone(),
            Self::None => vec![],
            Self::Text(s) => s.as_bytes().to_vec(),
        }
    }
}

impl From<String> for DataType {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}
impl From<Vec<u8>> for DataType {
    fn from(value: Vec<u8>) -> Self {
        Self::Binary(BinaryWrapper { inner: value })
    }
}

#[serde_with::serde_as]
#[derive(Debug, Deserialize, Serialize)]
#[serde(transparent)]
pub struct BinaryWrapper {
    #[serde_as(as = "serde_with::hex::Hex")]
    pub inner: Vec<u8>,
}

impl Request {
    pub fn parse(bytes: &[u8]) -> anyhow::Result<Self> {
        let a: Vec<u8> = vec![];
        let (headers, bytes) = parse_header(bytes)?;
        let (target_type, bytes) = parse_target_type(bytes)?;
        let (target, bytes) = parse_string(bytes)?;
        let (topic, bytes) = parse_string(bytes)?;
        let data = bytes.to_vec().into();

        let target = match target_type {
            0x01 => Target::Contract(target),
            0x02 => Target::Host(target),
            ty => {
                return Err(anyhow::anyhow!("Unsupported target type '{ty}'"))
            }
        };

        Ok(Self {
            headers,
            target,
            topic,
            data,
        })
    }
    pub async fn from_request(
        req: hyper::Request<hyper::Body>,
    ) -> anyhow::Result<(Self, bool)> {
        // HTTP REQUEST
        let (parts, req_body) = req.into_parts();
        println!("1");
        let is_binary = parts
            .headers
            .get(CONTENT_TYPE)
            .and_then(|h| {
                h.to_str().ok().map(|s| s.starts_with(CONTENT_TYPE_BINARY))
            })
            .unwrap_or_default();

        println!("2");
        let headers = parts
            .headers
            .iter()
            .filter_map(|(k, v)| {
                let a = v.as_bytes();
                serde_json::from_slice::<serde_json::Value>(a)
                    .ok()
                    .map(|v| (k.to_string(), v))
            })
            .collect();
        println!("{headers:?}");

        let paths: Vec<_> = parts
            .uri
            .path()
            .split('/')
            .skip_while(|p| p.is_empty())
            .collect();

        println!("{paths:?}");
        let target_type = paths
            .first()
            .ok_or_else(|| anyhow::anyhow!("Missing target type"))?;
        let target_type = target_type.parse()?;
        let target = paths
            .get(1)
            .ok_or_else(|| anyhow::anyhow!("Missing target"))?
            .to_string();
        println!("{target}");

        let target = match target_type {
            0x01 => Target::Contract(target),
            0x02 => Target::Host(target),
            ty => {
                return Err(anyhow::anyhow!("Unsupported target type '{ty}'"))
            }
        };
        println!("{target:?}");

        let topic = paths
            .get(2)
            .ok_or_else(|| anyhow::anyhow!("Missing topic"))?
            .to_string();
        println!("{topic}");

        let body = hyper::body::to_bytes(req_body).await?;

        let data = match is_binary {
            true => body.to_vec().into(),
            false => serde_json::from_slice::<DataType>(&body)
                .map_err(|e| anyhow::anyhow!("Invalid data {e}"))?,
        };
        println!("decoded");
        Ok((
            Request {
                headers,
                target,
                data: data,
                topic,
            },
            is_binary,
        ))
    }
}
const CONTENT_TYPE: &str = "Content-Type";
const CONTENT_TYPE_BINARY: &str = "application/octet-stream";

// impl TryFrom<hyper::Request<hyper::Body>> for Request {
//     type Error = anyhow::Error;
//     async fn try_from(req: hyper::Request<hyper::Body>) ->
// anyhow::Result<Self> {         // HTTP REQUEST
//         let (parts, req_body) = req.into_parts();
//         let body = hyper::body::to_bytes(req_body).await?;

//         let is_binary = parts
//             .headers
//             .get(CONTENT_TYPE)
//             .and_then(|h| {
//                 h.to_str().ok().map(|s| s.starts_with(CONTENT_TYPE_BINARY))
//             })
//             .unwrap_or_default();

//         if !is_binary {
//             return serde_json::from_slice(&body);
//         }
//         let headers = parts
//             .headers
//             .iter()
//             .filter_map(|(k, v)| {
//                 v.to_str().ok().map(|v| (k.to_string(), v.to_string()))
//             })
//             .collect();

//         let paths: Vec<_> = parts.uri.path().split_terminator('/').collect();
//         let target_type: u8 = paths.get(0).try_into()?;
//         let target = paths.get(1)?;
//         let target = match target_type {
//             0x01 => Target::Contract(target),
//             0x02 => Target::Host(target),
//             ty => {
//                 return Err(anyhow::anyhow!("Unsupported target type '{ty}'"))
//             }
//         };

//         let (topic, bytes) = parse_string(&body)?;
//         let data = bytes.to_vec().into();
//         Ok(Request {
//             headers,
//             target,
//             data,
//             topic,
//         })
//     }
// }

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
