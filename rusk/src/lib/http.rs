// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#[cfg(feature = "chain")]
mod chain;
mod event;
#[cfg(feature = "prover")]
mod prover;
#[cfg(feature = "chain")]
mod rusk;
mod stream;

pub(crate) use event::{
    DataType, ExecutionError, MessageResponse as EventResponse,
};

use tokio::task::JoinError;
use tracing::{debug, info, warn};

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;

use tokio::net::ToSocketAddrs;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::{io, task};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use http_body_util::Full;
use hyper::http::{HeaderName, HeaderValue};
use hyper::service::Service;
use hyper::{
    body::{Bytes, Incoming},
    HeaderMap, Method, Request, Response, StatusCode,
};
use hyper_tungstenite::{tungstenite, HyperWebsocket};
use hyper_util::server::conn::auto::Builder as HttpBuilder;

use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::protocol::{CloseFrame, Message};

use futures_util::SinkExt;

use hyper_util::rt::TokioIo;

#[cfg(feature = "node")]
use node_data::events::contract::ContractEvent;

use crate::http::event::FullOrStreamBody;
use crate::VERSION;

pub use self::event::{RuesDispatchEvent, RuesEvent, RUES_LOCATION_PREFIX};

use self::event::{ResponseData, RuesEventUri, SessionId};
use self::stream::Listener;

const RUSK_VERSION_HEADER: &str = "Rusk-Version";
const RUSK_VERSION_STRICT_HEADER: &str = "Rusk-Version-Strict";

pub struct HttpServer {
    handle: task::JoinHandle<()>,
    _shutdown: broadcast::Sender<Infallible>,
}

pub struct HttpServerConfig {
    pub address: String,
    pub cert: Option<PathBuf>,
    pub key: Option<PathBuf>,
    pub headers: HeaderMap,
    pub ws_event_channel_cap: usize,
}

impl HttpServer {
    pub async fn wait(self) -> Result<(), JoinError> {
        self.handle.await
    }

    pub async fn bind<A, H, P1, P2>(
        handler: H,
        event_receiver: broadcast::Receiver<RuesEvent>,
        ws_event_channel_cap: usize,
        addr: A,
        headers: HeaderMap,
        cert_and_key: Option<(P1, P2)>,
    ) -> io::Result<(Self, SocketAddr)>
    where
        A: ToSocketAddrs,
        H: HandleRequest,
        P1: AsRef<Path>,
        P2: AsRef<Path>,
    {
        let listener = match cert_and_key {
            Some(cert_and_key) => Listener::bind_tls(addr, cert_and_key).await,
            None => Listener::bind(addr).await,
        }?;

        let (shutdown_sender, shutdown_receiver) = broadcast::channel(1);

        let local_addr = listener.local_addr()?;

        info!("Starting HTTP Listener to {local_addr}");

        let handle = task::spawn(listening_loop(
            handler,
            listener,
            event_receiver,
            shutdown_receiver,
            headers,
            ws_event_channel_cap,
        ));

        let server = Self {
            handle,
            _shutdown: shutdown_sender,
        };
        Ok((server, local_addr))
    }
}

#[derive(Default)]
pub struct DataSources {
    pub sources: Vec<Box<dyn HandleRequest>>,
}

#[async_trait]
impl HandleRequest for DataSources {
    fn can_handle_rues(&self, event: &RuesDispatchEvent) -> bool {
        self.sources.iter().any(|s| s.can_handle_rues(event))
    }

    async fn handle_rues(
        &self,
        event: &RuesDispatchEvent,
    ) -> anyhow::Result<ResponseData> {
        info!("Received event at {}", event.uri);
        event.check_rusk_version()?;
        for h in &self.sources {
            if h.can_handle_rues(event) {
                return h.handle_rues(event).await;
            }
        }
        Err(anyhow::anyhow!("unsupported location"))
    }
}

#[derive(Clone)]
struct TokioExecutor;

impl<F> hyper::rt::Executor<F> for TokioExecutor
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    fn execute(&self, fut: F) {
        task::spawn(fut);
    }
}

async fn listening_loop<H>(
    handler: H,
    listener: Listener,
    events: broadcast::Receiver<RuesEvent>,
    mut shutdown: broadcast::Receiver<Infallible>,
    headers: HeaderMap,
    ws_event_channel_cap: usize,
) where
    H: HandleRequest,
{
    let sources = Arc::new(handler);
    let sockets_map = Arc::new(RwLock::new(HashMap::new()));

    let service = ExecutionService {
        sources: sources.clone(),
        sockets_map: sockets_map.clone(),
        events: events.resubscribe(),
        shutdown: shutdown.resubscribe(),
        headers: Arc::new(headers),
        ws_event_channel_cap,
    };

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .thread_name("http")
        .enable_all()
        .build()
        .expect("http runtime to be created");
    loop {
        tokio::select! {
            _ = shutdown.recv() => {
                runtime.shutdown_background();
                break;
            }
            r = listener.accept() => {
                let stream = match r {
                    Ok(stream) => stream,
                    Err(_) => break,
                };

                let http = HttpBuilder::new(TokioExecutor);

                let stream = TokioIo::new(stream);
                let service = service.clone();

                runtime.spawn(async move {
                    let conn = http.serve_connection_with_upgrades(stream, service);
                    conn.await
                });
            }
        }
    }
}

struct ExecutionService<H> {
    sources: Arc<H>,
    sockets_map:
        Arc<RwLock<HashMap<SessionId, mpsc::Sender<SubscriptionAction>>>>,
    events: broadcast::Receiver<RuesEvent>,
    shutdown: broadcast::Receiver<Infallible>,
    headers: Arc<HeaderMap>,
    ws_event_channel_cap: usize,
}

impl<H> Clone for ExecutionService<H> {
    fn clone(&self) -> Self {
        Self {
            sources: self.sources.clone(),
            sockets_map: self.sockets_map.clone(),
            events: self.events.resubscribe(),
            shutdown: self.shutdown.resubscribe(),
            headers: self.headers.clone(),
            ws_event_channel_cap: self.ws_event_channel_cap,
        }
    }
}

impl<H> Service<Request<Incoming>> for ExecutionService<H>
where
    H: HandleRequest,
{
    type Response = Response<FullOrStreamBody>;
    type Error = Infallible;
    type Future = Pin<
        Box<
            dyn Future<Output = Result<Self::Response, Self::Error>>
                + Send
                + 'static,
        >,
    >;

    /// Handle the HTTP request.
    ///
    /// A request may be a "normal" request, or a WebSocket upgrade request. In
    /// the former case, the request is handled on the spot, while in the
    /// latter task running the stream handler loop is spawned.
    fn call(&self, req: Request<Incoming>) -> Self::Future {
        let sources = self.sources.clone();
        let sockets_map = self.sockets_map.clone();
        let events = self.events.resubscribe();
        let shutdown = self.shutdown.resubscribe();
        let ws_event_channel_cap = self.ws_event_channel_cap;
        let headers = self.headers.clone();

        Box::pin(async move {
            let rsp = handle_request(
                req,
                sources,
                sockets_map,
                events,
                shutdown,
                ws_event_channel_cap,
            )
            .await;

            // We insert all the custom headers set in the configuration here,
            // skipping the ones that are invalid.
            rsp.map(|mut rsp| {
                rsp.headers_mut().extend(headers.as_ref().clone());
                rsp
            })
            .or_else(|error| {
                Ok(response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    error.to_string(),
                )
                .expect("Failed to build response"))
            })
        })
    }
}

enum SubscriptionAction {
    Subscribe(RuesEventUri),
    Unsubscribe(RuesEventUri),
}

async fn handle_stream_rues(
    sid: SessionId,
    websocket: HyperWebsocket,
    events: broadcast::Receiver<RuesEvent>,
    mut subscriptions: mpsc::Receiver<SubscriptionAction>,
    mut shutdown: broadcast::Receiver<Infallible>,
    sockets_map: Arc<
        RwLock<HashMap<SessionId, mpsc::Sender<SubscriptionAction>>>,
    >,
) {
    let mut stream = match websocket.await {
        Ok(stream) => stream,
        Err(_) => return,
    };

    if stream.send(Message::Text(sid.to_string())).await.is_err() {
        let _ = stream
            .close(Some(CloseFrame {
                code: CloseCode::Error,
                reason: Cow::from("Failed sending session ID"),
            }))
            .await;
        return;
    }

    let mut subscription_set = HashSet::new();

    let mut events = BroadcastStream::new(events);

    loop {
        tokio::select! {
            recv = stream.next() => {
                match recv {
                    Some(Ok(Message::Close(msg))) => {
                        debug!("Closing stream for {sid} due to {msg:?}");
                        let _ = stream.close(msg).await;
                        break;
                    }
                    Some(Err(e)) => {
                        let _ = stream.close(Some(CloseFrame {
                            code: CloseCode::Error,
                            reason: Cow::from("Internal error"),
                        })).await;
                        warn!("Closing stream for {sid} due to {e}");
                        break;
                    }
                    None => {
                        let _ = stream.close(Some(CloseFrame {
                            code: CloseCode::Error,
                            reason: Cow::from("No more events"),
                        })).await;
                        warn!("Closing stream for {sid} due to no more events");
                        break;
                    }
                    _ => {}
                }
            }
            _ = shutdown.recv() => {
                let _ = stream.close(Some(CloseFrame {
                    code: CloseCode::Away,
                    reason: Cow::from("Shutting down"),
                })).await;
                break;
            }

            subscription = subscriptions.recv() => {
                let subscription = match subscription {
                    Some(subscription) => subscription,
                    None => {
                        // If the subscription channel is closed, it means the server has stopped
                        // communicating with this loop, so we should inform the client and stop.
                        let _ = stream.close(Some(CloseFrame {
                            code: CloseCode::Away,
                            reason: Cow::from("Shutting down"),
                        })).await;
                        break;
                    },
                };

                match subscription {
                    SubscriptionAction::Subscribe(subscription) => {
                        subscription_set.insert(subscription);
                    },
                    SubscriptionAction::Unsubscribe(subscription) => {
                        subscription_set.remove(&subscription);
                    },
                }
            }

            Some(event) = events.next() => {
                let mut event = match event {
                    Ok(event) => event,
                    Err(_) => {
                        // If the event channel is closed, it means the
                        // server has stopped producing events, so we
                        // should inform the client and stop.
                        let _ = stream.close(Some(CloseFrame {
                            code: CloseCode::Away,
                            reason: Cow::from("Shutting down"),
                        })).await;
                        break;

                    }
                };

                // The event is subscribed to if it matches any of the subscriptions.
                let mut is_subscribed = false;
                for sub in &subscription_set {
                    if sub.matches(&event) {
                        is_subscribed = true;
                        break;
                    }
                }

                // If the event is subscribed, we send it to the client.
                if is_subscribed {
                    event.add_header("Content-Location", event.uri.to_string());
                    let event = event.to_bytes();

                    // If the event fails sending we close the socket on the client
                    // and stop processing further.
                    if stream.send(Message::Binary(event)).await.is_err() {
                        let _ = stream.close(Some(CloseFrame {
                            code: CloseCode::Error,
                            reason: Cow::from("Failed sending event"),
                        })).await;
                        break;
                    }
                }
            }
        }
    }

    let mut sockets = sockets_map.write().await;
    sockets.remove(&sid);
}

fn response(
    status: StatusCode,
    body: impl Into<Bytes>,
) -> Result<Response<FullOrStreamBody>, ExecutionError> {
    Ok(Response::builder()
        .status(status)
        .header(RUSK_VERSION_HEADER, VERSION.as_str())
        .body(Full::new(body.into()).into())
        .expect("Failed to build response"))
}

async fn handle_request_rues<H: HandleRequest>(
    mut req: Request<Incoming>,
    handler: Arc<H>,
    sockets_map: Arc<
        RwLock<HashMap<SessionId, mpsc::Sender<SubscriptionAction>>>,
    >,
    events: broadcast::Receiver<RuesEvent>,
    shutdown: broadcast::Receiver<Infallible>,
    ws_event_channel_cap: usize,
) -> Result<Response<FullOrStreamBody>, ExecutionError> {
    if hyper_tungstenite::is_upgrade_request(&req) {
        let (subscription_sender, subscriptions) =
            mpsc::channel(ws_event_channel_cap);

        let (response, websocket) = hyper_tungstenite::upgrade(&mut req, None)?;

        let mut sockets = sockets_map.write().await;

        // This is a new WebSocket connection, so we generate a new random ID
        // and create a new channel for it.
        let mut sid = rand::random();
        while sockets.contains_key(&sid) {
            sid = rand::random();
        }
        sockets.insert(sid, subscription_sender);

        task::spawn(handle_stream_rues(
            sid,
            websocket,
            events,
            subscriptions,
            shutdown,
            sockets_map.clone(),
        ));

        Ok(response.map(Into::into))
    } else if req.method() == Method::POST {
        let (event, binary_resp) = RuesDispatchEvent::from_request(req).await?;
        let _is_binary = event.is_binary();
        let mut resp_headers = event.x_headers();
        let (responder, mut receiver) = mpsc::unbounded_channel();
        handle_execution_rues(handler, event, responder).await;

        let execution_response = receiver
            .recv()
            .await
            .expect("An execution should always return a response");
        resp_headers.extend(execution_response.headers.clone());
        let mut resp = execution_response.into_http(binary_resp)?;

        for (k, v) in resp_headers {
            let k = HeaderName::from_str(&k)?;
            let v = match v {
                serde_json::Value::String(s) => HeaderValue::from_str(&s),
                serde_json::Value::Null => HeaderValue::from_str(""),
                _ => HeaderValue::from_str(&v.to_string()),
            }?;
            resp.headers_mut().append(k, v);
        }

        Ok(resp)
    } else {
        let sid = match SessionId::parse_from_req(&req) {
            None => {
                return response(
                    StatusCode::FAILED_DEPENDENCY,
                    "{\"error\":\"Session ID not provided or invalid\"}",
                );
            }
            Some(sid) => sid,
        };

        let uri = match RuesEventUri::parse_from_path(req.uri().path()) {
            None => {
                return response(
                    StatusCode::NOT_FOUND,
                    "{{\"error\":\"Invalid URL path\n\"}}",
                );
            }
            Some(s) => s,
        };

        let action_sender = match sockets_map.read().await.get(&sid) {
            Some(sender) => sender.clone(),
            None => {
                return response(
                    StatusCode::FAILED_DEPENDENCY,
                    "{\"error\":\"Session ID not provided or invalid\"}",
                );
            }
        };

        let action = match *req.method() {
            Method::GET => SubscriptionAction::Subscribe(uri),
            Method::DELETE => SubscriptionAction::Unsubscribe(uri),
            _ => {
                return response(
                    StatusCode::METHOD_NOT_ALLOWED,
                    "{\"error\":\"Method not allowed\"}",
                );
            }
        };

        if action_sender.send(action).await.is_err() {
            return response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "{\"error\":\"Failed consuming request\"}",
            );
        }

        response(StatusCode::OK, "")
    }
}

async fn handle_request<H>(
    req: Request<Incoming>,
    sources: Arc<H>,
    sockets_map: Arc<
        RwLock<HashMap<SessionId, mpsc::Sender<SubscriptionAction>>>,
    >,
    events: broadcast::Receiver<RuesEvent>,
    shutdown: broadcast::Receiver<Infallible>,
    ws_event_channel_cap: usize,
) -> Result<Response<FullOrStreamBody>, ExecutionError>
where
    H: HandleRequest,
{
    let path = req.uri().path();

    // If the request is a RUES request, we handle it differently.
    if path.starts_with(RUES_LOCATION_PREFIX) {
        return handle_request_rues(
            req,
            sources.clone(),
            sockets_map,
            events,
            shutdown,
            ws_event_channel_cap,
        )
        .await;
    }

    #[cfg(feature = "http-wasm")]
    if path == "/static/drivers/wallet-core.wasm" {
        let wallet_wasm = include_bytes!("../assets/wallet_core-1.0.0.wasm");
        let mut response =
            Response::new(Full::from(wallet_wasm.to_vec()).into());
        response.headers_mut().append(
            "Content-Type",
            HeaderValue::from_static("application/wasm"),
        );
        return Ok(response);
    }

    Err(ExecutionError::Generic(anyhow::anyhow!("Unsupported path")))
}

async fn handle_execution_rues<H>(
    sources: Arc<H>,
    event: RuesDispatchEvent,
    responder: mpsc::UnboundedSender<EventResponse>,
) where
    H: HandleRequest,
{
    let mut rsp = sources
        .handle_rues(&event)
        .await
        .map(|data| {
            let (data, mut headers) = data.into_inner();
            headers.append(&mut event.x_headers());
            EventResponse {
                data,
                error: None,
                headers,
            }
        })
        .unwrap_or_else(|e| EventResponse {
            headers: event.x_headers(),
            data: DataType::None,
            error: Some(e.to_string()),
        });

    rsp.set_header(RUSK_VERSION_HEADER, serde_json::json!(*VERSION));
    let _ = responder.send(rsp);
}

#[async_trait]
pub trait HandleRequest: Send + Sync + 'static {
    fn can_handle_rues(&self, request: &RuesDispatchEvent) -> bool;
    async fn handle_rues(
        &self,
        request: &RuesDispatchEvent,
    ) -> anyhow::Result<ResponseData>;
}

#[cfg(test)]
mod tests {
    use std::{fs, thread};

    use super::*;

    use dusk_core::abi::ContractId;
    use event::{BinaryWrapper, RequestData};
    use node_data::events::contract::{ContractEvent, ContractTxEvent};
    use std::net::TcpStream;
    use tungstenite::client;

    /// A [`HandleRequest`] implementation that returns the same data
    struct TestHandle;

    const STREAMED_DATA: &[&[u8; 16]] = &[
        b"I am call data 0",
        b"I am call data 1",
        b"I am call data 2",
        b"I am call data 3",
    ];

    #[async_trait]
    impl HandleRequest for TestHandle {
        fn can_handle_rues(&self, _: &RuesDispatchEvent) -> bool {
            true
        }
        async fn handle_rues(
            &self,
            request: &RuesDispatchEvent,
        ) -> anyhow::Result<ResponseData> {
            let response = match request.uri.inner() {
                ("test", _, "stream") => {
                    let (sender, rec) = std::sync::mpsc::channel();
                    thread::spawn(move || {
                        for f in STREAMED_DATA.iter() {
                            sender.send(f.to_vec()).unwrap()
                        }
                    });
                    ResponseData::new(rec)
                }
                ("test", _, "echo") => {
                    ResponseData::new(request.data.as_bytes().to_vec())
                }
                _ => anyhow::bail!("Unsupported"),
            };
            Ok(response)
        }
    }

    #[tokio::test]
    async fn http_query() {
        let cert_and_key: Option<(String, String)> = None;

        let (_, event_receiver) = broadcast::channel(16);
        let ws_event_channel_cap = 2;

        let (_server, local_addr) = HttpServer::bind(
            TestHandle,
            event_receiver,
            ws_event_channel_cap,
            "localhost:0",
            HeaderMap::new(),
            cert_and_key,
        )
        .await
        .expect("Binding the server to the address should succeed");

        let data = Vec::from(&b"I am call data 0"[..]);
        let data = RequestData::Binary(BinaryWrapper { inner: data });

        let request_bytes = data.as_bytes();

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{}/on/test/echo", local_addr))
            .body(request_bytes.to_vec())
            .send()
            .await
            .expect("Requesting should succeed");

        let response_bytes =
            response.bytes().await.expect("There should be a response");
        let response_bytes =
            hex::decode(response_bytes).expect("data to be hex encoded");

        assert_eq!(
            request_bytes, response_bytes,
            "Data received the same as sent"
        );
    }

    #[tokio::test]
    async fn https_query() {
        let cert_path = "tests/assets/cert.pem";
        let key_path = "tests/assets/key.pem";

        let cert_bytes = fs::read(cert_path).expect("cert file should exist");
        let certificate = reqwest::tls::Certificate::from_pem(&cert_bytes)
            .expect("cert should be valid");

        let (_, event_receiver) = broadcast::channel(16);
        let ws_event_channel_cap = 2;

        let (_server, local_addr) = HttpServer::bind(
            TestHandle,
            event_receiver,
            ws_event_channel_cap,
            "localhost:0",
            HeaderMap::new(),
            Some((cert_path, key_path)),
        )
        .await
        .expect("Binding the server to the address should succeed");

        let data = Vec::from(&b"I am call data 0"[..]);
        let data = RequestData::Binary(BinaryWrapper { inner: data });
        let request_bytes = data.as_bytes().to_vec();

        let client = reqwest::ClientBuilder::new()
            .add_root_certificate(certificate)
            .danger_accept_invalid_certs(true)
            .build()
            .expect("creating client should succeed");

        let response = client
            .post(format!(
                "https://localhost:{}/on/test/echo",
                local_addr.port()
            ))
            .body(request_bytes.clone())
            .send()
            .await
            .expect("Requesting should succeed");

        let response_bytes =
            response.bytes().await.expect("There should be a response");
        let response_bytes =
            hex::decode(response_bytes).expect("data to be hex encoded");

        assert_eq!(
            request_bytes, response_bytes,
            "Data received the same as sent"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn websocket_rues() {
        let cert_and_key: Option<(String, String)> = None;

        let (event_sender, event_receiver) = broadcast::channel(16);
        let ws_event_channel_cap = 2;

        let (_server, local_addr) = HttpServer::bind(
            TestHandle,
            event_receiver,
            ws_event_channel_cap,
            "localhost:0",
            HeaderMap::new(),
            cert_and_key,
        )
        .await
        .expect("Binding the server to the address should succeed");

        let stream = TcpStream::connect(local_addr)
            .expect("Connecting to the server should succeed");

        let ws_uri = format!("ws://{local_addr}/on");
        let (mut stream, _) = client(ws_uri, stream)
            .expect("Handshake with the server should succeed");

        let first_message =
            stream.read().expect("Session ID should be received");
        let sid = SessionId::parse(
            &first_message
                .into_text()
                .expect("Session ID should come in a text message"),
        )
        .expect("Session ID should be parsed");

        const SUB_CONTRACT_ID: ContractId = ContractId::from_bytes([1; 32]);
        const MAYBE_SUB_CONTRACT_ID: ContractId =
            ContractId::from_bytes([2; 32]);
        const NON_SUB_CONTRACT_ID: ContractId = ContractId::from_bytes([3; 32]);

        const TOPIC: &str = "topic";

        let sub_contract_id_hex = hex::encode(SUB_CONTRACT_ID);
        let maybe_sub_contract_id_hex = hex::encode(MAYBE_SUB_CONTRACT_ID);

        let client = reqwest::Client::new();

        let response = client
            .get(format!(
                "http://{local_addr}/on/contracts:{sub_contract_id_hex}/{TOPIC}",
            ))
            .header("Rusk-Session-Id", sid.to_string())
            .send()
            .await
            .expect("Requesting should succeed");

        assert_eq!(response.status(), StatusCode::OK);

        let response = client
            .get(format!(
                "http://{local_addr}/on/contracts:{maybe_sub_contract_id_hex}/{TOPIC}",
            ))
            .header("Rusk-Session-Id", sid.to_string())
            .send()
            .await
            .expect("Requesting should succeed");

        assert_eq!(response.status(), StatusCode::OK);

        // This event is subscribed to, so it should be received
        let received_event = RuesEvent::from(ContractTxEvent {
            event: ContractEvent {
                target: SUB_CONTRACT_ID,
                topic: TOPIC.into(),
                data: b"hello, events".to_vec(),
            },
            origin: [0; 32],
        });

        // This event is at first subscribed to, so it should be received the
        // first time
        let at_first_received_event = RuesEvent::from(ContractTxEvent {
            event: ContractEvent {
                target: MAYBE_SUB_CONTRACT_ID,
                topic: TOPIC.into(),
                data: b"hello, events".to_vec(),
            },
            origin: [1; 32],
        });

        // This event is not subscribed to, so it should not be received
        let non_received_event = RuesEvent::from(ContractTxEvent {
            event: ContractEvent {
                target: NON_SUB_CONTRACT_ID,
                topic: TOPIC.into(),
                data: b"hello, events".to_vec(),
            },
            origin: [2; 32],
        });

        event_sender
            .send(non_received_event.clone())
            .expect("Sending event should succeed");

        event_sender
            .send(at_first_received_event.clone())
            .expect("Sending event should succeed");

        event_sender
            .send(received_event.clone())
            .expect("Sending event should succeed");

        let message = stream.read().expect("Event should be received");
        let event_bytes = message.into_data();

        let event = from_bytes(&event_bytes).expect("Event should deserialize");

        assert_eq!(at_first_received_event, event, "Event should be the same");

        let message = stream.read().expect("Event should be received");
        let event_bytes = message.into_data();

        let event = from_bytes(&event_bytes).expect("Event should deserialize");

        assert_eq!(received_event, event, "Event should be the same");

        let response = client
            .delete(format!(
                "http://{local_addr}/on/contracts:{maybe_sub_contract_id_hex}/{TOPIC}",
            ))
            .header("Rusk-Session-Id", sid.to_string())
            .send()
            .await
            .expect("Requesting should succeed");

        assert_eq!(response.status(), StatusCode::OK);

        event_sender
            .send(non_received_event.clone())
            .expect("Sending event should succeed");

        event_sender
            .send(at_first_received_event.clone())
            .expect("Sending event should succeed");

        event_sender
            .send(received_event.clone())
            .expect("Sending event should succeed");

        let message = stream.read().expect("Event should be received");

        let event_bytes = message.into_data();

        let event = from_bytes(&event_bytes).expect("Event should deserialize");

        assert_eq!(received_event, event, "Event should be the same");
    }

    fn parse_len(bytes: &[u8]) -> anyhow::Result<(usize, &[u8])> {
        if bytes.len() < 4 {
            return Err(anyhow::anyhow!("not enough bytes"));
        }

        let len = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
            as usize;
        let (_, left) = bytes.split_at(4);

        Ok((len, left))
    }

    type Header<'a> = (serde_json::Map<String, serde_json::Value>, &'a [u8]);
    pub(crate) fn parse_header(bytes: &[u8]) -> anyhow::Result<Header> {
        let (len, bytes) = parse_len(bytes)?;
        if bytes.len() < len {
            return Err(anyhow::anyhow!(
                "not enough bytes for parsed len {len}"
            ));
        }

        let (header_bytes, bytes) = bytes.split_at(len);
        let header = serde_json::from_slice(header_bytes)?;

        Ok((header, bytes))
    }

    pub fn from_bytes(data: &[u8]) -> anyhow::Result<RuesEvent> {
        let (mut headers, data) = parse_header(data)?;

        let path = headers
            .remove("Content-Location")
            .ok_or(anyhow::anyhow!("Content location is not set"))?
            .as_str()
            .ok_or(anyhow::anyhow!("Content location is not a string"))?
            .to_string();

        let uri = RuesEventUri::parse_from_path(&path)
            .ok_or(anyhow::anyhow!("Invalid location"))?;

        let data = data.to_vec().into();
        Ok(RuesEvent { data, headers, uri })
    }
}
