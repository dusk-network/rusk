// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![allow(unused)]

mod chain;
mod event;
mod rusk;

pub(crate) use event::{
    BinaryWrapper, Event as EventRequest, ExecutionError,
    MessageResponse as EventResponse, RequestData, ResponseData, Target,
};
use hyper::http::{HeaderName, HeaderValue};
use tracing::info;

use std::borrow::Cow;
use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use std::task::{Context, Poll};

use async_trait::async_trait;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, ToSocketAddrs};
use tokio::sync::{broadcast, mpsc};
use tokio::{io, task};

use hyper::server::conn::Http;
use hyper::service::Service;
use hyper::{body, Body, Request, Response, StatusCode};
use hyper_tungstenite::{tungstenite, HyperWebsocket};

use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::protocol::{CloseFrame, Message};

use futures_util::{stream, SinkExt, StreamExt};

use crate::chain::RuskNode;
use crate::Rusk;

use self::event::MessageRequest;

pub struct HttpServer {
    handle: task::JoinHandle<()>,
    local_addr: SocketAddr,
    _shutdown: broadcast::Sender<Infallible>,
}

impl HttpServer {
    pub async fn bind<A: ToSocketAddrs, H: HandleRequest>(
        handler: H,
        addr: A,
    ) -> io::Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        let (shutdown_sender, shutdown_receiver) = broadcast::channel(1);

        let local_addr = listener.local_addr()?;

        info!("Starting HTTP Listener to {local_addr}");

        let handle =
            task::spawn(listening_loop(handler, listener, shutdown_receiver));

        Ok(Self {
            handle,
            local_addr,
            _shutdown: shutdown_sender,
        })
    }
}

pub struct DataSources {
    pub rusk: Rusk,
    pub node: RuskNode,
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
        match request.event.to_route() {
            (Target::Contract(_), ..) | (_, "rusk", _) => {
                self.rusk.handle_request(request).await
            }
            (_, "Chain", _) => self.node.handle_request(request).await,
            _ => Err(anyhow::anyhow!("unsupported target type")),
        }
    }
}

async fn listening_loop<H: HandleRequest>(
    handler: H,
    listener: TcpListener,
    mut shutdown: broadcast::Receiver<Infallible>,
) {
    let handler = Arc::new(handler);
    let http = Http::new();

    loop {
        tokio::select! {
            _ = shutdown.recv() => {
                break;
            }
            r = listener.accept() => {
                if r.is_err() {
                    break;
                }
                let (stream, _) = r.unwrap();

                let service = ExecutionService {
                    sources: handler.clone(),
                    shutdown: shutdown.resubscribe()
                };
                let conn = http.serve_connection(stream, service).with_upgrades();

                task::spawn(conn);

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

                if let ResponseData::Channel(c) = rsp.data {
                    let mut datas = stream::iter(c).map(|e| {
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
    shutdown: broadcast::Receiver<Infallible>,
}

impl<H> Service<Request<Body>> for ExecutionService<H>
where
    H: HandleRequest,
{
    type Response = Response<Body>;
    type Error = Infallible;
    type Future = Pin<
        Box<
            dyn Future<Output = Result<Self::Response, Self::Error>>
                + Send
                + 'static,
        >,
    >;

    fn poll_ready(
        &mut self,
        _: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    /// Handle the HTTP request.
    ///
    /// A request may be a "normal" request, or a WebSocket upgrade request. In
    /// the former case, the request is handled on the spot, while in the
    /// latter task running the stream handler loop is spawned.
    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        let sources = self.sources.clone();
        let shutdown = self.shutdown.resubscribe();

        Box::pin(async move {
            let response = handle_request(req, shutdown, sources).await;
            response.or_else(|error| {
                Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::from(error.to_string()))
                    .expect("Failed to build response"))
            })
        })
    }
}

async fn handle_request<H>(
    mut req: Request<Body>,
    mut shutdown: broadcast::Receiver<Infallible>,
    sources: Arc<H>,
) -> Result<Response<Body>, ExecutionError>
where
    H: HandleRequest,
{
    if hyper_tungstenite::is_upgrade_request(&req) {
        let target = req.uri().path().try_into()?;
        let (response, websocket) = hyper_tungstenite::upgrade(&mut req, None)?;
        task::spawn(handle_stream(sources, websocket, target, shutdown));

        Ok(response)
    } else {
        let (execution_request, is_binary) =
            MessageRequest::from_request(req).await?;

        let x_headers = execution_request.x_headers();

        let (responder, mut receiver) = mpsc::unbounded_channel();
        handle_execution(sources, execution_request, responder).await;

        let execution_response = receiver
            .recv()
            .await
            .expect("An execution should always return a response");
        let mut resp = execution_response.into_http(is_binary)?;

        for (k, v) in x_headers {
            let k = HeaderName::from_str(&k)?;
            let v = HeaderValue::from_str(&v.to_string())?;
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
    let rsp = sources
        .handle(&request)
        .await
        .map(|data| EventResponse {
            data,
            error: None,
            headers: request.x_headers(),
        })
        .unwrap_or_else(|e| request.to_error(e.to_string()));

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
    use std::thread;

    use super::*;

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
                    ResponseData::Channel(rec)
                }
                _ => request.event_data().to_vec().into(),
            };
            Ok(response)
        }
    }

    #[tokio::test]
    async fn http_query() {
        let server = HttpServer::bind(TestHandle, "localhost:0")
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
            .body(Body::from(request))
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
        let server = HttpServer::bind(TestHandle, "localhost:0")
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
        let headers: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(r#"{"X-requestid": "100"}"#)
                .expect("headers to be serialized");

        let request = MessageRequest {
            event,
            headers: headers.clone(),
        };

        let request = serde_json::to_string(&request).unwrap();

        stream
            .write_message(Message::Text(request))
            .expect("Sending request to the server should succeed");

        let mut responses = vec![];
        // Vec::<ExecutionResponse>::with_capacity(request_num);

        while responses.len() < STREAMED_DATA.len() {
            let msg = stream
                .read_message()
                .expect("Response should be received without error");

            let msg = match msg {
                Message::Text(msg) => msg,
                _ => panic!("Shouldn't receive anything but text"),
            };
            let response: EventResponse = serde_json::from_str(&msg)
                .expect("Response should deserialize successfully");
            assert_eq!(
                response.headers, headers,
                "x- headers to be propagated back"
            );
            assert!(matches!(response.error, None), "There should be noerror");
            match response.data {
                ResponseData::Binary(BinaryWrapper { inner }) => {
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
}
