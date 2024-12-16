// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::RUSK_VERSION_HEADER;

use base64::engine::{general_purpose::STANDARD as BASE64, Engine};
use bytecheck::CheckBytes;
use dusk_core::ContractId;
use futures_util::stream::Iter as StreamIter;
use futures_util::{stream, Stream, StreamExt};
use http_body_util::{BodyExt, Either, Full, StreamBody};
use hyper::body::{Buf, Frame};
use hyper::header::{InvalidHeaderName, InvalidHeaderValue};
use hyper::{
    body::{Body, Bytes, Incoming},
    Request, Response,
};
use pin_project::pin_project;
use rand::distributions::{Distribution, Standard};
use rand::Rng;
use rkyv::Archive;
use semver::{Version, VersionReq};
use serde::de::{Error, MapAccess, Unexpected, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::pin::Pin;
use std::str::FromStr;
use std::str::Split;
use std::sync::mpsc;
use std::task::{Context, Poll};
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
        req: Request<Incoming>,
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
    ) -> anyhow::Result<Response<FullOrStreamBody>> {
        if let Some(error) = &self.error {
            return Ok(hyper::Response::builder()
                .status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
                .body(Full::new(error.to_string().into()).into())?);
        }

        let mut headers = HashMap::new();

        let body = {
            match self.data {
                DataType::Binary(wrapper) => {
                    let data = match is_binary {
                        true => wrapper.inner,
                        false => hex::encode(wrapper.inner).as_bytes().to_vec(),
                    };
                    Full::from(Bytes::from(data)).into()
                }
                DataType::Text(text) => Full::from(Bytes::from(text)).into(),
                DataType::Json(value) => {
                    headers.insert(CONTENT_TYPE, CONTENT_TYPE_JSON.clone());
                    Full::from(Bytes::from(value.to_string())).into()
                }
                DataType::Channel(receiver) => FullOrStreamBody {
                    either: Either::Right(StreamBody::new(
                        BinaryOrTextStream {
                            is_binary,
                            stream: stream::iter(receiver),
                        },
                    )),
                },
                DataType::None => Full::new(Bytes::new()).into(),
            }
        };
        let mut response = Response::new(body);
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

#[pin_project]
pub struct FullOrStreamBody {
    #[pin]
    either: Either<Full<Bytes>, StreamBody<BinaryOrTextStream>>,
}

impl From<Full<Bytes>> for FullOrStreamBody {
    fn from(body: Full<Bytes>) -> Self {
        Self {
            either: Either::Left(body),
        }
    }
}

impl Body for FullOrStreamBody {
    type Data =
        <Either<Full<Bytes>, StreamBody<BinaryOrTextStream>> as Body>::Data;
    type Error =
        <Either<Full<Bytes>, StreamBody<BinaryOrTextStream>> as Body>::Error;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let this = self.project();
        this.either.poll_frame(cx)
    }
}

#[pin_project]
pub struct BinaryOrTextStream {
    is_binary: bool,
    #[pin]
    stream: StreamIter<<mpsc::Receiver<Vec<u8>> as IntoIterator>::IntoIter>,
}

impl Stream for BinaryOrTextStream {
    type Item = anyhow::Result<Frame<Bytes>>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.project();
        this.stream.poll_next(cx).map(|next| {
            next.map(|x| match this.is_binary {
                true => Ok(Frame::data(Bytes::from(x))),
                false => Ok(Frame::data(Bytes::from(
                    hex::encode(x).as_bytes().to_vec(),
                ))),
            })
        })
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

#[derive(Debug, Clone, Eq, PartialEq)]
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

    pub fn add_header<K: Into<String>, V: Into<serde_json::Value>>(
        &mut self,
        key: K,
        value: V,
    ) {
        self.header.insert(key.into(), value.into());
    }

    pub fn with_header<K: Into<String>, V: Into<serde_json::Value>>(
        mut self,
        key: K,
        value: V,
    ) -> Self {
        self.add_header(key, value);
        self
    }

    pub fn into_inner(
        self,
    ) -> (DataType, serde_json::Map<String, serde_json::Value>) {
        (self.data, self.header)
    }

    pub fn data(&self) -> &DataType {
        &self.data
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

impl Eq for DataType {}

impl PartialEq for DataType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Channel(_), Self::Channel(_)) => true,
            (Self::Text(a), Self::Text(b)) => a == b,
            (Self::Json(a), Self::Json(b)) => a == b,
            (Self::Binary(a), Self::Binary(b)) => a == b,
            (Self::None, Self::None) => true,
            _ => false,
        }
    }
}

impl Clone for DataType {
    fn clone(&self) -> Self {
        match self {
            Self::Binary(b) => b.inner.clone().into(),
            Self::Text(s) => s.clone().into(),
            Self::Json(v) => v.clone().into(),
            _ => Self::None,
        }
    }
}

impl DataType {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Self::Binary(b) => b.inner.clone(),
            Self::Text(s) => s.as_bytes().to_vec(),
            Self::Json(s) => s.to_string().as_bytes().to_vec(),
            _ => vec![],
        }
    }
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
#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
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
        req: Request<Incoming>,
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

        let body = req_body.collect().await?.to_bytes();

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
const CONTENT_TYPE: &str = "content-type";
const ACCEPT: &str = "accept";
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
pub(crate) fn parse_header(bytes: &[u8]) -> anyhow::Result<Header> {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId(u128);

impl Display for SessionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let bytes = self.0.to_le_bytes();
        let hex = hex::encode(bytes);
        write!(f, "{hex}")
    }
}

impl Distribution<SessionId> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> SessionId {
        SessionId(rng.gen())
    }
}

impl SessionId {
    /// Parses a session ID from a request. The session ID is expected to be
    /// stored in the `Rusk-Session-Id` header.
    pub fn parse_from_req<B>(req: &Request<B>) -> Option<Self> {
        let headers = req.headers();

        let header_value = headers.get("Rusk-Session-Id")?;
        let text = header_value.to_str().ok()?;

        Self::parse(text)
    }

    pub fn parse(text: &str) -> Option<Self> {
        let bytes = hex::decode(text).ok()?;

        let mut session_id_bytes = [0u8; 16];
        if bytes.len() != 16 {
            return None;
        }

        session_id_bytes.copy_from_slice(&bytes);
        Some(SessionId(u128::from_le_bytes(session_id_bytes)))
    }
}

/// A subscription to an event.
///
/// Subscriptions have data related to the component the subscriber wishes to
/// subscribe to, the component targeted by the event (`contracts`,
/// `transactions`, etc...) and an optional entity within the component that
/// the event targets.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct RuesEventUri {
    pub component: String,
    pub entity: Option<String>,
    pub topic: String,
}

pub const RUES_LOCATION_PREFIX: &str = "/on";

impl Display for RuesEventUri {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let component = &self.component;
        let entity = self
            .entity
            .as_ref()
            .map(|e| format!(":{e}"))
            .unwrap_or_default();
        let topic = &self.topic;

        write!(f, "{RUES_LOCATION_PREFIX}/{component}{entity}/{topic}")
    }
}

impl RuesEventUri {
    pub fn inner(&self) -> (&str, Option<&String>, &str) {
        (
            self.component.as_ref(),
            self.entity.as_ref(),
            self.topic.as_ref(),
        )
    }

    pub fn parse_from_path(path: &str) -> Option<Self> {
        if !path.starts_with(RUES_LOCATION_PREFIX) {
            return None;
        }
        // Skip '/on' since we already know its present
        let path = &path[RUES_LOCATION_PREFIX.len()..];

        let mut path_split = path.split('/');

        // Skip first '/'
        path_split.next()?;

        // If the segment contains a `:`, we split the string in two after the
        // first one - meaning entities with `:` are still possible.
        // If the segment doesn't contain a `:` then the segment is just a
        // component.
        let (component, entity) =
            path_split
                .next()
                .map(|segment| match segment.split_once(':') {
                    Some((component, entity)) => (component, Some(entity)),
                    None => (segment, None),
                })?;

        let component = component.to_string().to_lowercase();
        let entity = entity.map(ToString::to_string);
        let topic = path_split.next()?.to_string().to_lowercase();

        Some(Self {
            component,
            entity,
            topic,
        })
    }

    pub fn matches(&self, event: &RuesEvent) -> bool {
        let event = &event.uri;
        if self.component != event.component {
            return false;
        }

        if self.entity.is_some() && self.entity != event.entity {
            return false;
        }

        if self.topic != event.topic {
            return false;
        }
        true
    }
}

/// A RUES event
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RuesEvent {
    pub uri: RuesEventUri,
    pub headers: serde_json::Map<String, serde_json::Value>,
    pub data: DataType,
}

/// A RUES Dispatch request event
#[derive(Debug)]
pub struct RuesDispatchEvent {
    pub uri: RuesEventUri,
    pub headers: serde_json::Map<String, serde_json::Value>,
    pub data: RequestData,
}

impl RuesDispatchEvent {
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

    pub fn is_binary(&self) -> bool {
        self.headers
            .get(CONTENT_TYPE)
            .and_then(|h| h.as_str())
            .map(|v| v.eq_ignore_ascii_case(CONTENT_TYPE_BINARY))
            .unwrap_or_default()
    }
    pub async fn from_request(
        req: Request<Incoming>,
    ) -> anyhow::Result<(Self, bool)> {
        let (parts, body) = req.into_parts();

        let uri = RuesEventUri::parse_from_path(parts.uri.path())
            .ok_or(anyhow::anyhow!("Invalid URL path"))?;

        let headers = parts
            .headers
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

        // HTTP REQUEST
        let content_type = parts
            .headers
            .get(CONTENT_TYPE)
            .and_then(|h| h.to_str().ok())
            .unwrap_or_default();

        let binary_request = content_type == CONTENT_TYPE_BINARY;

        let binary_response = binary_request
            || parts
                .headers
                .get(ACCEPT)
                .and_then(|h| h.to_str().ok())
                .map(|v| v.eq_ignore_ascii_case(CONTENT_TYPE_BINARY))
                .unwrap_or_default();

        let bytes = body.collect().await?.to_bytes().to_vec();
        let data = match binary_request {
            true => bytes.into(),
            _ => {
                let text = String::from_utf8(bytes)
                    .map_err(|e| anyhow::anyhow!("Invalid utf8"))?;
                if let Some(hex) = text.strip_prefix("0x") {
                    if let Ok(bytes) = hex::decode(hex) {
                        bytes.into()
                    } else {
                        text.into()
                    }
                } else {
                    text.into()
                }
            }
        };

        let ret = RuesDispatchEvent { headers, data, uri };

        Ok((ret, binary_response))
    }
}

impl RuesEvent {
    pub fn add_header<K: Into<String>, V: Into<serde_json::Value>>(
        &mut self,
        key: K,
        value: V,
    ) {
        self.headers.insert(key.into(), value.into());
    }

    /// Serialize the event into a vector of bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        let headers_bytes = serde_json::to_vec(&self.headers)
            .expect("Serializing JSON should succeed");

        let headers_len = headers_bytes.len() as u32;
        let headers_len_bytes = headers_len.to_le_bytes();

        let data_bytes = self.data.to_bytes();

        let len =
            headers_len_bytes.len() + headers_bytes.len() + data_bytes.len();
        let mut bytes = Vec::with_capacity(len);

        bytes.extend(headers_len_bytes);
        bytes.extend(headers_bytes);
        bytes.extend(data_bytes);

        bytes
    }
}

#[cfg(feature = "chain")]
impl From<node_data::events::contract::ContractTxEvent> for RuesEvent {
    fn from(tx_event: node_data::events::contract::ContractTxEvent) -> Self {
        let mut headers = serde_json::Map::new();

        headers
            .insert("Rusk-Origin".into(), hex::encode(tx_event.origin).into());

        let event = tx_event.event;
        Self {
            uri: RuesEventUri {
                component: "contracts".into(),
                entity: Some(hex::encode(event.target.0)),
                topic: event.topic,
            },
            data: event.data.into(),
            headers,
        }
    }
}

#[cfg(feature = "chain")]
impl From<node_data::events::Event> for RuesEvent {
    fn from(value: node_data::events::Event) -> Self {
        let data = value.data.map_or(DataType::None, DataType::Json);

        Self {
            uri: RuesEventUri {
                component: value.component.into(),
                entity: Some(value.entity),
                topic: value.topic.into(),
            },
            data,
            headers: Default::default(),
        }
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
