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

use std::borrow::Cow;
use std::convert::Infallible;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use std::task::{Context, Poll};

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
    pub async fn bind<A: ToSocketAddrs>(
        rusk: Rusk,
        node: RuskNode,
        addr: A,
    ) -> io::Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        let (shutdown_sender, shutdown_receiver) = broadcast::channel(1);

        let local_addr = listener.local_addr()?;

        let handle = task::spawn(listening_loop(
            DataSources { rusk, node },
            listener,
            shutdown_receiver,
        ));

        Ok(Self {
            handle,
            local_addr,
            _shutdown: shutdown_sender,
        })
    }
}

struct DataSources {
    rusk: Rusk,
    node: RuskNode,
}

async fn listening_loop(
    sources: DataSources,
    listener: TcpListener,
    mut shutdown: broadcast::Receiver<Infallible>,
) {
    let sources = Arc::new(sources);
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
                    sources: sources.clone(),
                    shutdown: shutdown.resubscribe()
                };
                let conn = http.serve_connection(stream, service).with_upgrades();

                task::spawn(conn);

            }
        }
    }
}

async fn handle_stream(
    sources: Arc<DataSources>,
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

struct ExecutionService {
    sources: Arc<DataSources>,
    shutdown: broadcast::Receiver<Infallible>,
}

impl Service<Request<Body>> for ExecutionService {
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

async fn handle_request(
    mut req: Request<Body>,
    mut shutdown: broadcast::Receiver<Infallible>,
    sources: Arc<DataSources>,
) -> Result<Response<Body>, ExecutionError> {
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

async fn handle_execution(
    sources: Arc<DataSources>,
    request: MessageRequest,
    responder: mpsc::UnboundedSender<EventResponse>,
) {
    let rsp = match request.event.target {
        Target::Contract(_) => sources.rusk.handle_request(request).await,
        Target::Host(_) => sources.node.handle_request(request).await,
        _ => EventResponse {
            headers: request.x_headers(),
            data: event::ResponseData::None,
            error: Some("unsupported target type".into()),
        },
    };

    let _ = responder.send(rsp);
}
