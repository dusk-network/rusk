// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![allow(unused)]

#[cfg(feature = "node")]
mod chain;
mod event;
#[cfg(feature = "prover")]
mod prover;
#[cfg(feature = "node")]
mod rusk;
mod stream;

pub(crate) use event::{
    BinaryWrapper, DataType, ExecutionError, MessageResponse as EventResponse,
    RequestData, Target,
};
use rusk_abi::Event;
use tracing::{info, warn};

use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::path::Path;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::mpsc as std_mpsc;
use std::sync::Arc;
use std::task::{Context, Poll};

use async_trait::async_trait;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::ToSocketAddrs;
use tokio::sync::{broadcast, mpsc, oneshot, RwLock};
use tokio::{io, task};
use tokio_stream::wrappers::{BroadcastStream, ReceiverStream};
use tokio_stream::StreamExt;
use tokio_util::either::Either;

use http_body_util::Full;
use hyper::http::{HeaderName, HeaderValue};
use hyper::service::Service;
use hyper::{
    body::{self, Body, Bytes, Incoming},
    Method, Request, Response, StatusCode,
};
use hyper_tungstenite::{tungstenite, HyperWebsocket};
use hyper_util::server::conn::auto::Builder as HttpBuilder;

use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::protocol::{CloseFrame, Message};

use futures_util::stream::iter as stream_iter;
use futures_util::{SinkExt, TryStreamExt};

use anyhow::Error as AnyhowError;
use hyper_util::rt::TokioIo;
use rand::rngs::OsRng;

#[cfg(feature = "node")]
use crate::chain::{Rusk, RuskNode};
use crate::http::event::FullOrStreamBody;
use crate::VERSION;

pub use self::event::ContractEvent;
use self::event::{MessageRequest, ResponseData, RuesSubscription, SessionId};
use self::stream::{Listener, Stream};

const RUSK_VERSION_HEADER: &str = "Rusk-Version";

pub struct HttpServer {
    pub handle: task::JoinHandle<()>,
    local_addr: SocketAddr,
    pub _shutdown: broadcast::Sender<Infallible>,
}

impl HttpServer {
    pub async fn bind<A, H, P1, P2>(
        handler: H,
        event_receiver: broadcast::Receiver<ContractEvent>,
        ws_event_channel_cap: usize,
        addr: A,
        cert_and_key: Option<(P1, P2)>,
    ) -> io::Result<Self>
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
            ws_event_channel_cap,
        ));

        Ok(Self {
            handle,
            local_addr,
            _shutdown: shutdown_sender,
        })
    }
}

pub struct DataSources {
    #[cfg(feature = "node")]
    pub rusk: Rusk,
    #[cfg(feature = "node")]
    pub node: RuskNode,
    #[cfg(feature = "prover")]
    pub prover: rusk_prover::LocalProver,
}

#[async_trait]
impl HandleRequest for DataSources {
    async fn handle(
        &self,
        request: &MessageRequest,
    ) -> anyhow::Result<ResponseData> {
        info!(
            "Received {:?}:{} request",
            request.event.target, request.event.topic
        );
        request.check_rusk_version()?;
        match request.event.to_route() {
            #[cfg(feature = "prover")]
            // target `rusk` shall be removed in future versions
            (_, "rusk", topic) | (_, "prover", topic)
                if topic.starts_with("prove_") =>
            {
                self.prover.handle(request).await
            }
            #[cfg(feature = "node")]
            (Target::Contract(_), ..) | (_, "rusk", _) => {
                self.rusk.handle(request).await
            }
            #[cfg(feature = "node")]
            (_, "Chain", _) => self.node.handle(request).await,
            _ => Err(anyhow::anyhow!("unsupported target type")),
        }
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
    events: broadcast::Receiver<ContractEvent>,
    mut shutdown: broadcast::Receiver<Infallible>,
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
        ws_event_channel_cap,
    };

    loop {
        tokio::select! {
            _ = shutdown.recv() => {
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

                task::spawn(async move {
                    let conn = http.serve_connection_with_upgrades(stream, service);
                    conn.await
                });
            }
        }
    }
}

async fn handle_stream<H: HandleRequest>(
    sources: Arc<H>,
    websocket: HyperWebsocket,
    target: Target,
    mut shutdown: broadcast::Receiver<Infallible>,
) {
    let mut stream = match websocket.await {
        Ok(stream) => stream,
        Err(_) => return,
    };

    // Add this block to disable requests through websockets
    // {
    //     let _ = stream
    //         .close(Some(CloseFrame {
    //             code: CloseCode::Unsupported,
    //             reason: Cow::from("Websocket is currently unsupported"),
    //         }))
    //         .await;
    //     #[allow(clippy::needless_return)]
    //     return;
    // }

    let (responder, mut responses) = mpsc::unbounded_channel::<EventResponse>();

    'outer: loop {
        tokio::select! {
            // If the server shuts down we send a close frame to the client
            // and stop.
            _ = shutdown.recv() => {
                let _ = stream.close(Some(CloseFrame {
                    code: CloseCode::Away,
                    reason: Cow::from("Shutting down"),
                })).await;
                break;
            }

            rsp = responses.recv() => {
                // `responder` is never dropped so this can never be `None`
                let rsp = rsp.unwrap();

                if let DataType::Channel(c) = rsp.data {
                    let mut datas = stream_iter(c).map(|e| {
                        EventResponse {
                            data: e.into(),
                            headers: rsp.headers.clone(),
                            error: None
                        }
                    });//.await;
                    while let Some(c) = datas.next().await {
                        let rsp = serde_json::to_string(&c).unwrap_or_else(|err| {
                            serde_json::to_string(
                                &EventResponse::from_error(
                                    format!("Failed serializing response: {err}")
                                )).expect("serializing error response should succeed")
                            });

                        // If we error in sending the message we send a close frame
                        // to the client and stop.
                        if stream.send(Message::Text(rsp)).await.is_err() {
                            let _ = stream.close(Some(CloseFrame {
                            code: CloseCode::Error,
                            reason: Cow::from("Failed sending response"),
                            })).await;
                            // break;
                        }
                    }


                } else {
                    // Serialize the response to text. If this does not succeed,
                    // we simply serialize an error response.
                    let rsp = serde_json::to_string(&rsp).unwrap_or_else(|err| {
                        serde_json::to_string(
                            &EventResponse::from_error(
                                format!("Failed serializing response: {err}")
                            )).expect("serializing error response should succeed")
                        });

                    // If we error in sending the message we send a close frame
                    // to the client and stop.
                    if stream.send(Message::Text(rsp)).await.is_err() {
                        let _ = stream.close(Some(CloseFrame {
                        code: CloseCode::Error,
                        reason: Cow::from("Failed sending response"),
                        })).await;
                        break;
                    }
                }
            }

            msg = stream.next() => {

                let mut req = match msg {
                    Some(Ok(msg)) => match msg {
                        // We received a text request.
                        Message::Text(msg) => {
                            serde_json::from_str(&msg)
                                .map_err(|err| anyhow::anyhow!("Failed deserializing request: {err}"))
                        },
                        // We received a binary request.
                        Message::Binary(msg) => {
                            MessageRequest::parse(&msg)
                                .map_err(|err| anyhow::anyhow!("Failed deserializing request: {err}"))
                        }
                        // Any other type of message is unsupported.
                        _ => Err(anyhow::anyhow!("Only text and binary messages are supported"))
                    }
                    // Errored while receiving the message, we will
                    // close the stream and return a close frame.
                    Some(Err(err)) => {
                        Err(anyhow::anyhow!("Failed receiving message: {err}"))
                    }
                    // The stream has stopped producing messages, and we
                    // should close it and stop. The client likely has done
                    // this on purpose, and it's a part of the normal
                    // operation of the server.
                    None => {
                        let _ = stream.close(Some(CloseFrame {
                            code: CloseCode::Normal,
                            reason: Cow::from("Stream stopped"),
                        })).await;
                        break;
                    }
                };
                match req {
                    // We received a valid request and should spawn a new task to handle it
                    Ok(mut req) => {
                        req.event.target=target.clone();
                        task::spawn(handle_execution(
                            sources.clone(),
                            req,
                            responder.clone(),
                        ));
                    },
                    Err(e) => {
                        let _ = stream.close(Some(CloseFrame {
                            code: CloseCode::Error,
                            reason: Cow::from(e.to_string()),
                        })).await;
                        break;
                    }
                }

            }
        }
    }
}

struct ExecutionService<H> {
    sources: Arc<H>,
    sockets_map:
        Arc<RwLock<HashMap<SessionId, mpsc::Sender<SubscriptionAction>>>>,
    events: broadcast::Receiver<ContractEvent>,
    shutdown: broadcast::Receiver<Infallible>,
    ws_event_channel_cap: usize,
}

impl<H> Clone for ExecutionService<H> {
    fn clone(&self) -> Self {
        Self {
            sources: self.sources.clone(),
            sockets_map: self.sockets_map.clone(),
            events: self.events.resubscribe(),
            shutdown: self.shutdown.resubscribe(),
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
    fn call(&self, mut req: Request<Incoming>) -> Self::Future {
        let sources = self.sources.clone();
        let sockets_map = self.sockets_map.clone();
        let events = self.events.resubscribe();
        let shutdown = self.shutdown.resubscribe();
        let ws_event_channel_cap = self.ws_event_channel_cap;

        Box::pin(async move {
            let response = handle_request(
                req,
                sources,
                sockets_map,
                events,
                shutdown,
                ws_event_channel_cap,
            )
            .await;
            response.map(Into::into).or_else(|error| {
                Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Full::new(error.to_string().into()).into())
                    .expect("Failed to build response"))
            })
        })
    }
}

enum SubscriptionAction {
    Subscribe(RuesSubscription),
    Unsubscribe(RuesSubscription),
    Dispatch {
        sub: RuesSubscription,
        body: Incoming,
    },
}

async fn handle_stream_rues(
    sid: SessionId,
    websocket: HyperWebsocket,
    mut subscriptions: mpsc::Receiver<SubscriptionAction>,
    events: broadcast::Receiver<ContractEvent>,
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

    // FIXME make this a configuration parameter
    const DISPATCH_BUFFER_SIZE: usize = 16;

    let mut subscription_set = HashSet::new();
    let (dispatch_sender, dispatch_events) =
        mpsc::channel(DISPATCH_BUFFER_SIZE);

    // Join the two event receivers together, allowing for reusing the exact
    // same code when handling them either of them.
    let mut events = BroadcastStream::new(events);
    let mut dispatch_events = ReceiverStream::new(dispatch_events);

    let mut events = events
        .map_err(Either::Left)
        .merge(dispatch_events.map_err(Either::Right));

    loop {
        tokio::select! {
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
                    SubscriptionAction::Dispatch {
                        sub,
                        body
                    } => {
                        // TODO figure out if we should subscribe to the event we dispatch
                        task::spawn(handle_dispatch(sub, body, dispatch_sender.clone()));
                    }
                }
            }

            Some(event) = events.next() => {
                let event = match event {
                    Ok(event) => event,
                    Err(err) => match err {
                        Either::Left(_berr) => {
                            // If the event channel is closed, it means the
                            // server has stopped producing events, so we
                            // should inform the client and stop.
                            let _ = stream.close(Some(CloseFrame {
                                code: CloseCode::Away,
                                reason: Cow::from("Shutting down"),
                            })).await;
                            break;

                        }
                        Either::Right(_eerr) => {
                            // TODO handle execution error
                            continue;
                        },
                    },
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
                    let event = match serde_json::to_string(&event) {
                        Ok(event) => event,
                        // If we fail to serialize the event, we log the error
                        // and continue processing further.
                        Err(err) => {
                            warn!("Failed serializing event: {err}");
                            continue;
                        }
                    };

                    // If the event fails sending we close the socket on the client
                    // and stop processing further.
                    if stream.send(Message::Text(event)).await.is_err() {
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

async fn handle_dispatch(
    sub: RuesSubscription,
    body: Incoming,
    sender: mpsc::Sender<Result<ContractEvent, AnyhowError>>,
) {
    todo!(
        "\
    Figure out if the subscription is a contract subscription (meaning a \
    contract call) and, if so, parse the body for the arguments and execute, \
    giving somehow passing the resulting events through to the websocket stream
    that dispatched the event.
    "
    )
}

fn response(
    status: StatusCode,
    body: impl Into<Bytes>,
) -> Result<Response<FullOrStreamBody>, ExecutionError> {
    Ok(Response::builder()
        .status(status)
        .body(Full::new(body.into()).into())
        .expect("Failed to build response"))
}

async fn handle_request_rues(
    mut req: Request<Incoming>,
    sockets_map: Arc<
        RwLock<HashMap<SessionId, mpsc::Sender<SubscriptionAction>>>,
    >,
    events: broadcast::Receiver<ContractEvent>,
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
            subscriptions,
            events,
            shutdown,
            sockets_map.clone(),
        ));

        Ok(response.map(Into::into))
    } else {
        let headers = req.headers();
        let mut path_split = req.uri().path().split('/');

        // Skip '/on' since we already know its present
        path_split.next();
        path_split.next();

        let sid = match SessionId::parse_from_req(&req) {
            None => {
                return response(
                    StatusCode::FAILED_DEPENDENCY,
                    "{\"error\":\"Session ID not provided or invalid\"}",
                );
            }
            Some(sid) => sid,
        };

        let subscription =
            match RuesSubscription::parse_from_path_split(path_split) {
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
            Method::GET => SubscriptionAction::Subscribe(subscription),
            Method::DELETE => SubscriptionAction::Unsubscribe(subscription),
            Method::POST => SubscriptionAction::Dispatch {
                sub: subscription,
                body: req.into_body(),
            },
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
    mut req: Request<Incoming>,
    sources: Arc<H>,
    sockets_map: Arc<
        RwLock<HashMap<SessionId, mpsc::Sender<SubscriptionAction>>>,
    >,
    events: broadcast::Receiver<ContractEvent>,
    shutdown: broadcast::Receiver<Infallible>,
    ws_event_channel_cap: usize,
) -> Result<Response<FullOrStreamBody>, ExecutionError>
where
    H: HandleRequest,
{
    let path = req.uri().path();

    // If the request is a RUES request, we handle it differently.
    if path.starts_with("/on") {
        return handle_request_rues(
            req,
            sockets_map,
            events,
            shutdown,
            ws_event_channel_cap,
        )
        .await;
    }

    if hyper_tungstenite::is_upgrade_request(&req) {
        let target = req.uri().path().try_into()?;

        let (response, websocket) = hyper_tungstenite::upgrade(&mut req, None)?;
        task::spawn(handle_stream(sources, websocket, target, shutdown));

        Ok(response.map(Into::into))
    } else {
        let (execution_request, binary_resp) =
            MessageRequest::from_request(req).await?;

        let mut resp_headers = execution_request.x_headers();

        let (responder, mut receiver) = mpsc::unbounded_channel();
        handle_execution(sources, execution_request, responder).await;

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
    }
}

async fn handle_execution<H>(
    sources: Arc<H>,
    request: MessageRequest,
    responder: mpsc::UnboundedSender<EventResponse>,
) where
    H: HandleRequest,
{
    let mut rsp = sources
        .handle(&request)
        .await
        .map(|data| {
            let (data, mut headers) = data.into_inner();
            headers.append(&mut request.x_headers());
            EventResponse {
                data,
                error: None,
                headers,
            }
        })
        .unwrap_or_else(|e| request.to_error(e.to_string()));

    rsp.set_header(RUSK_VERSION_HEADER, serde_json::json!(*VERSION));
    let _ = responder.send(rsp);
}

#[async_trait]
pub trait HandleRequest: Send + Sync + 'static {
    async fn handle(
        &self,
        request: &MessageRequest,
    ) -> anyhow::Result<ResponseData>;
}

#[cfg(test)]
mod tests {
    use std::{fs, thread};

    use super::*;
    use event::Event as EventRequest;

    use crate::http::event::WrappedContractId;
    use rusk_abi::ContractId;
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
        async fn handle(
            &self,
            request: &MessageRequest,
        ) -> anyhow::Result<ResponseData> {
            let response = match request.event.to_route() {
                (_, _, "stream") => {
                    let (sender, rec) = std::sync::mpsc::channel();
                    thread::spawn(move || {
                        for f in STREAMED_DATA.iter() {
                            sender.send(f.to_vec()).unwrap()
                        }
                    });
                    ResponseData::new(rec)
                }
                _ => ResponseData::new(request.event_data().to_vec()),
            };
            Ok(response)
        }
    }

    #[tokio::test]
    async fn http_query() {
        let cert_and_key: Option<(String, String)> = None;

        let (_, event_receiver) = broadcast::channel(16);
        let ws_event_channel_cap = 2;

        let server = HttpServer::bind(
            TestHandle,
            event_receiver,
            ws_event_channel_cap,
            "localhost:0",
            cert_and_key,
        )
        .await
        .expect("Binding the server to the address should succeed");

        let data = Vec::from(&b"I am call data 0"[..]);
        let data = RequestData::Binary(BinaryWrapper { inner: data });

        let event = EventRequest {
            target: Target::None,
            data,
            topic: "topic".into(),
        };

        let request = serde_json::to_vec(&event)
            .expect("Serializing request should succeed");

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{}/01/target", server.local_addr))
            .body(request)
            .send()
            .await
            .expect("Requesting should succeed");

        let response_bytes =
            response.bytes().await.expect("There should be a response");
        let response_bytes =
            hex::decode(response_bytes).expect("data to be hex encoded");
        let request_bytes = event.data.as_bytes();

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

        let server = HttpServer::bind(
            TestHandle,
            event_receiver,
            ws_event_channel_cap,
            "localhost:0",
            Some((cert_path, key_path)),
        )
        .await
        .expect("Binding the server to the address should succeed");

        let data = Vec::from(&b"I am call data 0"[..]);
        let data = RequestData::Binary(BinaryWrapper { inner: data });

        let event = EventRequest {
            target: Target::None,
            data,
            topic: "topic".into(),
        };

        let request = serde_json::to_vec(&event)
            .expect("Serializing request should succeed");

        let client = reqwest::ClientBuilder::new()
            .add_root_certificate(certificate)
            .danger_accept_invalid_certs(true)
            .build()
            .expect("creating client should succeed");

        let response = client
            .post(format!(
                "https://localhost:{}/01/target",
                server.local_addr.port()
            ))
            .body(request)
            .send()
            .await
            .expect("Requesting should succeed");

        let response_bytes =
            response.bytes().await.expect("There should be a response");
        let response_bytes =
            hex::decode(response_bytes).expect("data to be hex encoded");
        let request_bytes = event.data.as_bytes();

        assert_eq!(
            request_bytes, response_bytes,
            "Data received the same as sent"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn websocket_queries() {
        let cert_and_key: Option<(String, String)> = None;

        let (_, event_receiver) = broadcast::channel(16);
        let ws_event_channel_cap = 2;

        let server = HttpServer::bind(
            TestHandle,
            event_receiver,
            ws_event_channel_cap,
            "localhost:0",
            cert_and_key,
        )
        .await
        .expect("Binding the server to the address should succeed");

        let stream = TcpStream::connect(server.local_addr)
            .expect("Connecting to the server should succeed");

        let ws_uri = format!("ws://{}/01/stream", server.local_addr);
        let (mut stream, _) = client(ws_uri, stream)
            .expect("Handshake with the server should succeed");

        let event = EventRequest {
            target: Target::None,
            data: RequestData::Text("Not used".into()),
            topic: "stream".into(),
        };
        let request_x_header: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(r#"{"X-requestid": "100"}"#)
                .expect("headers to be serialized");

        let request = MessageRequest {
            event,
            headers: request_x_header.clone(),
        };

        let request = serde_json::to_string(&request).unwrap();

        stream
            .send(Message::Text(request))
            .expect("Sending request to the server should succeed");

        let mut responses = vec![];

        while responses.len() < STREAMED_DATA.len() {
            let msg = stream
                .read()
                .expect("Response should be received without error");

            let msg = match msg {
                Message::Text(msg) => msg,
                _ => panic!("Shouldn't receive anything but text"),
            };
            let response: EventResponse = serde_json::from_str(&msg)
                .expect("Response should deserialize successfully");

            let mut response_x_header = response.headers.clone();
            response_x_header.retain(|k, _| k.to_lowercase().starts_with("x-"));
            assert_eq!(
                response_x_header, request_x_header,
                "x-headers to be propagated back"
            );
            assert!(response.error.is_none(), "There should be no error");
            match response.data {
                DataType::Binary(BinaryWrapper { inner }) => {
                    responses.push(inner);
                }
                _ => panic!("WS stream is supposed to return binary data"),
            }
        }

        for (idx, response) in responses.iter().enumerate() {
            let expected_data = STREAMED_DATA[idx];
            assert_eq!(
                &response[..],
                expected_data,
                "Response data should be the same as the request `fn_args`"
            );
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn websocket_rues() {
        let cert_and_key: Option<(String, String)> = None;

        let (event_sender, event_receiver) = broadcast::channel(16);
        let ws_event_channel_cap = 2;

        let server = HttpServer::bind(
            TestHandle,
            event_receiver,
            ws_event_channel_cap,
            "localhost:0",
            cert_and_key,
        )
        .await
        .expect("Binding the server to the address should succeed");

        let stream = TcpStream::connect(server.local_addr)
            .expect("Connecting to the server should succeed");

        let ws_uri = format!("ws://{}/on", server.local_addr);
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

        const SUB_CONTRACT_ID: WrappedContractId =
            WrappedContractId(ContractId::from_bytes([1; 32]));
        const MAYBE_SUB_CONTRACT_ID: WrappedContractId =
            WrappedContractId(ContractId::from_bytes([2; 32]));
        const NON_SUB_CONTRACT_ID: WrappedContractId =
            WrappedContractId(ContractId::from_bytes([3; 32]));

        const TOPIC: &str = "topic";

        let sub_contract_id_hex = hex::encode(SUB_CONTRACT_ID.0);
        let maybe_sub_contract_id_hex = hex::encode(MAYBE_SUB_CONTRACT_ID.0);

        let client = reqwest::Client::new();

        let response = client
            .get(format!(
                "http://{}/on/contracts:{sub_contract_id_hex}/{TOPIC}",
                server.local_addr
            ))
            .header("Rusk-Session-Id", sid.to_string())
            .send()
            .await
            .expect("Requesting should succeed");

        assert_eq!(response.status(), StatusCode::OK);

        let response = client
            .get(format!(
                "http://{}/on/contracts:{maybe_sub_contract_id_hex}/{TOPIC}",
                server.local_addr
            ))
            .header("Rusk-Session-Id", sid.to_string())
            .send()
            .await
            .expect("Requesting should succeed");

        assert_eq!(response.status(), StatusCode::OK);

        // This event is subscribed to, so it should be received
        let received_event = ContractEvent {
            target: SUB_CONTRACT_ID,
            topic: TOPIC.into(),
            data: b"hello, events".to_vec(),
        };

        // This event is at first subscribed to, so it should be received the
        // first time
        let at_first_received_event = ContractEvent {
            target: MAYBE_SUB_CONTRACT_ID,
            topic: TOPIC.into(),
            data: b"hello, events".to_vec(),
        };

        // This event is not subscribed to, so it should not be received
        let non_received_event = ContractEvent {
            target: NON_SUB_CONTRACT_ID,
            topic: TOPIC.into(),
            data: b"hello, events".to_vec(),
        };

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
        let event_text = message.into_text().expect("Event should be text");

        let event: ContractEvent = serde_json::from_str(&event_text)
            .expect("Event should deserialize");

        assert_eq!(at_first_received_event, event, "Event should be the same");

        let message = stream.read().expect("Event should be received");
        let event_text = message.into_text().expect("Event should be text");

        let event: ContractEvent = serde_json::from_str(&event_text)
            .expect("Event should deserialize");

        assert_eq!(received_event, event, "Event should be the same");

        let response = client
            .delete(format!(
                "http://{}/on/contracts:{maybe_sub_contract_id_hex}/{TOPIC}",
                server.local_addr
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
        let event_text = message.into_text().expect("Event should be text");

        let event: ContractEvent = serde_json::from_str(&event_text)
            .expect("Event should deserialize");

        assert_eq!(received_event, event, "Event should be the same");
    }
}
