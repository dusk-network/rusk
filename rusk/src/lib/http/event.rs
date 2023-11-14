// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::RUSK_VERSION_HEADER;
use futures_util::{stream, StreamExt};
use hyper::header::{InvalidHeaderName, InvalidHeaderValue};
use hyper::Body;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use serde_with::{self, serde_as};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use std::sync::mpsc;
use tungstenite::http::HeaderValue;

/// A request sent by the websocket client.
#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
    #[serde(skip)]
    pub target: Target,
    pub topic: String,
    pub data: RequestData,
}

impl Event {
    pub fn to_route(&self) -> (&Target, &str, &str) {
        (&self.target, self.target.inner(), self.topic.as_ref())
    }
}

/// A request sent by the websocket client.
#[derive(Debug, Serialize, Deserialize)]
pub struct MessageRequest {
    pub headers: serde_json::Map<String, serde_json::Value>,
    pub event: Event,
}

impl MessageRequest {
    pub fn to_error<S>(&self, err: S) -> MessageResponse
    where
        S: AsRef<str>,
    {
        MessageResponse {
            headers: self.x_headers(),
            data: DataType::None,
            error: Some(err.as_ref().to_string()),
        }
    }

    pub fn event_data(&self) -> &[u8] {
        self.event.data.as_bytes()
    }
}

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub enum Target {
    #[default]
    None,
    Contract(String), // 0x01
    Host(String),     // 0x02
    Debugger(String), // 0x03
}

impl Target {
    pub fn inner(&self) -> &str {
        match self {
            Self::None => "",
            Self::Contract(s) => s,
            Self::Host(s) => s,
            Self::Debugger(s) => s,
        }
    }
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
            .map(|(k, v)| {
                let v = if v.is_empty() {
                    serde_json::Value::Null
                } else {
                    serde_json::from_slice::<serde_json::Value>(v.as_bytes())
                        .unwrap_or(serde_json::Value::String(
                            v.to_str().unwrap().to_string(),
                        ))
                };
                (k.to_string().to_lowercase(), v)
            })
            .collect();
        let (event, binary_response) = Event::from_request(req).await?;

        let req = MessageRequest { event, headers };

        Ok((req, binary_response))
    }

    pub fn check_rusk_version(&self) -> anyhow::Result<()> {
        if let Some(v) = self.header(RUSK_VERSION_HEADER) {
            let req = match v.as_str() {
                Some(v) => VersionReq::from_str(v),
                None => VersionReq::from_str(&v.to_string()),
            }?;

            let current = Version::from_str(&crate::VERSION)?;
            if !req.matches(&current) {
                return Err(anyhow::anyhow!(
                    "Mismatched rusk version: requested {req} - current {current}",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MessageResponse {
    pub headers: serde_json::Map<String, serde_json::Value>,

    /// The data returned by the contract call.
    pub data: DataType,

    /// A possible error happening during the contract call.
    pub error: Option<String>,
}

impl MessageResponse {
    pub fn from_error(error: String) -> Self {
        Self {
            headers: serde_json::Map::default(),
            data: DataType::None,
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

        let mut headers = HashMap::new();

        let body = {
            match self.data {
                DataType::Binary(wrapper) => {
                    let data = match is_binary {
                        true => wrapper.inner,
                        false => hex::encode(wrapper.inner).as_bytes().to_vec(),
                    };
                    Body::from(data)
                }
                DataType::Text(text) => Body::from(text),
                DataType::Json(value) => {
                    headers.insert(CONTENT_TYPE, CONTENT_TYPE_JSON.clone());
                    Body::from(value.to_string())
                }
                DataType::Channel(channel) => {
                    Body::wrap_stream(stream::iter(channel).map(move |e| {
                        match is_binary {
                            true => Ok::<_, anyhow::Error>(e),
                            false => Ok::<_, anyhow::Error>(
                                hex::encode(e).as_bytes().to_vec(),
                            ),
                        }
                    }))
                }
                DataType::None => Body::empty(),
            }
        };
        let mut response = hyper::Response::new(body);
        for (k, v) in headers {
            response.headers_mut().insert(k, v);
        }
        Ok(response)
    }

    pub fn set_header(&mut self, key: &str, value: serde_json::Value) {
        // search for the key in a case-insensitive way
        let v = self
            .headers
            .iter_mut()
            .find_map(|(k, v)| k.eq_ignore_ascii_case(key).then_some(v));

        if let Some(v) = v {
            *v = value;
        } else {
            self.headers.insert(key.into(), value);
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestData {
    Binary(BinaryWrapper),
    Text(String),
}

impl RequestData {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Binary(w) => &w.inner,
            Self::Text(s) => s.as_bytes(),
        }
    }

    pub fn as_string(&self) -> String {
        match self {
            Self::Binary(w) => {
                String::from_utf8(w.inner.clone()).unwrap_or_default()
            }
            Self::Text(s) => s.clone(),
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

pub struct ResponseData {
    data: DataType,
    header: serde_json::Map<String, serde_json::Value>,
}

impl ResponseData {
    pub fn new<D: Into<DataType>>(data: D) -> Self {
        Self {
            data: data.into(),
            header: serde_json::Map::new(),
        }
    }

    pub fn with_header<K: Into<String>, V: Into<serde_json::Value>>(
        mut self,
        key: K,
        value: V,
    ) -> Self {
        self.header.insert(key.into(), value.into());
        self
    }

    pub fn into_inner(
        self,
    ) -> (DataType, serde_json::Map<String, serde_json::Value>) {
        (self.data, self.header)
    }
}

/// Data in a response.
#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(untagged)]
pub enum DataType {
    Binary(BinaryWrapper),
    Text(String),
    Json(serde_json::Value),
    #[serde(skip)]
    Channel(mpsc::Receiver<Vec<u8>>),
    #[default]
    None,
}

impl From<serde_json::Value> for DataType {
    fn from(value: serde_json::Value) -> Self {
        Self::Json(value)
    }
}

impl From<String> for DataType {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl From<Vec<u8>> for DataType {
    fn from(bytes: Vec<u8>) -> Self {
        Self::Binary(BinaryWrapper { inner: bytes })
    }
}

impl From<mpsc::Receiver<Vec<u8>>> for DataType {
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
        let binary_request = parts
            .headers
            .get(CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .map(|v| v.eq_ignore_ascii_case(CONTENT_TYPE_BINARY))
            .unwrap_or_default();

        let target = parts.uri.path().try_into()?;

        let body = hyper::body::to_bytes(req_body).await?;

        let mut event = match binary_request {
            true => Event::parse(&body)
                .map_err(|e| anyhow::anyhow!("Invalid data {e}"))?,
            false => serde_json::from_slice(&body)
                .map_err(|e| anyhow::anyhow!("Invalid data {e}"))?,
        };
        event.target = target;

        let binary_response = binary_request
            || parts
                .headers
                .get(ACCEPT)
                .and_then(|h| h.to_str().ok())
                .map(|v| v.eq_ignore_ascii_case(CONTENT_TYPE_BINARY))
                .unwrap_or_default();

        Ok((event, binary_response))
    }
}
const CONTENT_TYPE: &str = "Content-Type";
const ACCEPT: &str = "Accept";
const CONTENT_TYPE_BINARY: &str = "application/octet-stream";
static CONTENT_TYPE_JSON: HeaderValue =
    HeaderValue::from_static("application/json");

fn parse_len(bytes: &[u8]) -> anyhow::Result<(usize, &[u8])> {
    if bytes.len() < 4 {
        return Err(anyhow::anyhow!("not enough bytes"));
    }

    let len =
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    let (_, left) = bytes.split_at(4);

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

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn event() {
        let data =
            "120000006c65617665735f66726f6d5f6865696768740000000000000000";
        let data = hex::decode(data).unwrap();
        let event = Event::parse(&data).unwrap();
    }
}
