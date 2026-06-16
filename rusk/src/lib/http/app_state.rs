#![cfg_attr(
    not(any(feature = "chain", feature = "prover", test)),
    allow(dead_code)
)]

use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;

use axum::http::HeaderMap;
use tokio::sync::{RwLock, broadcast, mpsc};

use crate::http::rues::SubscriptionAction;
use crate::http::{HttpHandlers, HttpRequestPolicy, RuesEvent, SessionId};

#[derive(Clone)]
pub(crate) struct HttpAppState {
    pub services: HttpHandlers,
    pub sockets_map:
        Arc<RwLock<HashMap<SessionId, mpsc::Sender<SubscriptionAction>>>>,
    pub events: broadcast::Sender<RuesEvent>,
    pub shutdown: broadcast::Sender<Infallible>,
    pub ws_event_channel_cap: usize,
    pub enable_docs: bool,
    pub policy: Arc<HttpRequestPolicy>,
    pub headers: Arc<HeaderMap>,
}
