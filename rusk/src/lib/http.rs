// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::borrow::Cow;
use std::convert::Infallible;
use std::fmt::{Display, Formatter};
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures_util::{SinkExt, StreamExt};

use tokio::net::{TcpListener, ToSocketAddrs};
use tokio::sync::{broadcast, mpsc};
use tokio::{io, task};

use hyper::server::conn::Http;
use hyper::service::Service;
use hyper::{body, Body, Request, Response};
use hyper_tungstenite::{tungstenite, HyperWebsocket};

use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::protocol::{CloseFrame, Message};

use rusk_abi::ContractId;
use serde::{Deserialize, Serialize};

type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

/// Rusk's HTTP server implementation.
///
/// It allows for a client to request executions of the VM, and responds with
/// the product of said execution.
///
/// It also supports performing these operations over a long-lived websocket
/// connection.
pub struct HttpServer {
    handle: task::JoinHandle<()>,
    local_addr: SocketAddr,
    _shutdown: broadcast::Sender<Infallible>,
}

impl HttpServer {
    /// Bind the server to the given `addr`ess, using the given `vm` for
    /// querying.
    pub async fn bind<Q, A>(vm: Q, addr: A) -> io::Result<Self>
    where
        Q: QueryRaw,
        A: ToSocketAddrs,
    {
        let listener = TcpListener::bind(addr).await?;
        let (shutdown_sender, shutdown_receiver) = broadcast::channel(1);

        let local_addr = listener.local_addr()?;

        let handle = task::spawn(listening_loop(
            DataSources { vm },
            listener,
            shutdown_receiver,
        ));

        Ok(Self {
            handle,
            local_addr,
            _shutdown: shutdown_sender,
        })
    }

    /// Returns the address the server is listening on, or was listening on if
    /// it has shutdown.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Returns a reference to the listening loop's join handle.
    pub fn task_handle(&self) -> &task::JoinHandle<()> {
        &self.handle
    }
}

struct DataSources<VM> {
    vm: VM,
}

/// Accepts incoming streams and passes them to the [`ExecutionService`],
/// together with the given data sources.
///
/// When all [`mpsc::Sender`]s that are coupled to the `shutdown` receiver are
/// dropped, the loop will exit and all streams will be closed.
async fn listening_loop<VM>(
    sources: DataSources<VM>,
    listener: TcpListener,
    mut shutdown: broadcast::Receiver<Infallible>,
) where
    VM: QueryRaw,
{
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

/// Receives a stream, performs the WebSocket handshake on it, and keeps
/// receiving messages until either the stream closes, or the server shuts down.
///
/// If a message is received that the server cannot parse, then the stream will
/// be closed.
async fn handle_stream<VM>(
    sources: Arc<DataSources<VM>>,
    websocket: HyperWebsocket,
    mut shutdown: broadcast::Receiver<Infallible>,
) where
    VM: QueryRaw,
{
    let mut stream = match websocket.await {
        Ok(stream) => stream,
        Err(_) => return,
    };

    let (responder, mut responses) =
        mpsc::unbounded_channel::<ExecutionResponse>();

    loop {
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

                // Serialize the response to text. If this does not succeed,
                // we simply serialize an error response.
                let rsp = serde_json::to_string(&rsp).unwrap_or_else(|err| {
                    serde_json::to_string(&ExecutionResponse {
                        request_id: rsp.request_id,
                        data: vec![],
                        error: Some(format!("Failed serializing response: {err}")),
                    }).expect("serializing error response should succeed")
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

            msg = stream.next() => {
                match msg {
                    Some(Ok(msg)) => match msg {
                        // We received a request and should spawn a new task to handle it.
                        Message::Text(msg) => {
                            match serde_json::from_str(&msg) {
                                Ok(req) => {
                                    task::spawn(handle_execution(
                                        sources.clone(),
                                        req,
                                        responder.clone(),
                                    ));
                                },
                                Err(err) => {
                                    let _ = stream.close(Some(CloseFrame {
                                        code: CloseCode::Error,
                                        reason: Cow::from(format!("Failed deserializing request: {err}")),
                                    })).await;
                                    break;
                                }
                            }
                        }
                        // Any other type of message is unsupported.
                        _ => {
                            let _ = stream.close(Some(CloseFrame {
                                code: CloseCode::Unsupported,
                                reason: Cow::from("Only text messages are supported"),
                            })).await;
                            break;
                        },
                    }
                    // Errored while receiving the message, we will
                    // close the stream and return a close frame.
                    Some(Err(err)) => {
                        let _ = stream.close(Some(CloseFrame {
                            code: CloseCode::Error,
                            reason: Cow::from(format!("Failed receiving message: {err}")),
                        })).await;
                        break;
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
                }
            }
        }
    }
}

async fn handle_execution<VM>(
    sources: Arc<DataSources<VM>>,
    request: ExecutionRequest,
    responder: mpsc::UnboundedSender<ExecutionResponse>,
) where
    VM: QueryRaw,
{
    let vm = &sources.vm;

    let contract = ContractId::from_bytes(request.contract);

    let rsp = match vm.query_raw(contract, request.fn_name, request.fn_args) {
        Ok(data) => ExecutionResponse {
            request_id: request.request_id,
            data,
            error: None,
        },
        Err(err) => ExecutionResponse {
            request_id: request.request_id,
            data: vec![],
            error: Some(format!("Query error: {err}")),
        },
    };

    let _ = responder.send(rsp);
}

/// Something that accepts queries whose type information is unknown.
pub trait QueryRaw: 'static + Send + Sync {
    /// The error returned by the raw query.
    type Error: Display;

    /// Performs a raw query.
    fn query_raw<N, C>(
        &self,
        contract: ContractId,
        fn_name: N,
        fn_args: C,
    ) -> Result<Vec<u8>, Self::Error>
    where
        N: AsRef<str>,
        C: Into<Vec<u8>>;
}

impl QueryRaw for crate::Rusk {
    type Error = crate::Error;

    fn query_raw<N, C>(
        &self,
        contract: ContractId,
        fn_name: N,
        fn_args: C,
    ) -> Result<Vec<u8>, Self::Error>
    where
        N: AsRef<str>,
        C: Into<Vec<u8>>,
    {
        Self::query_raw(self, contract, fn_name, fn_args)
    }
}

struct ExecutionService<Q> {
    sources: Arc<DataSources<Q>>,
    shutdown: broadcast::Receiver<Infallible>,
}

impl<Q> Service<Request<Body>> for ExecutionService<Q>
where
    Q: QueryRaw,
{
    type Response = Response<Body>;
    type Error = ExecutionError;
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
            if hyper_tungstenite::is_upgrade_request(&req) {
                let (response, websocket) =
                    hyper_tungstenite::upgrade(&mut req, None)?;

                task::spawn(handle_stream(sources, websocket, shutdown));

                Ok(response)
            } else {
                let (_, req_body) = req.into_parts();

                let body = body::to_bytes(req_body).await?;
                let execution_request = serde_json::from_slice(&body)?;

                let (responder, mut receiver) = mpsc::unbounded_channel();
                handle_execution(sources, execution_request, responder).await;

                let execution_response = receiver
                    .recv()
                    .await
                    .expect("An execution should always return a response");

                let response_body = serde_json::to_vec(&execution_response)?;
                Ok(Response::new(Body::from(response_body)))
            }
        })
    }
}

/// A request sent by the websocket client, asking for a specific contract
/// function to be executed with the given arguments.
#[serde_with::serde_as]
#[derive(Debug, Deserialize, Serialize)]
struct ExecutionRequest {
    /// The request ID, allowing for differentiating multiple in-flight
    /// requests.
    request_id: Option<i32>,
    /// The contract to call.
    #[serde_as(as = "serde_with::hex::Hex")]
    contract: [u8; 32],
    /// The function name to call in the contract.
    fn_name: String,
    /// The arguments to pass to the function.
    #[serde_as(as = "serde_with::hex::Hex")]
    fn_args: Vec<u8>,
}

/// Response to a [`ExecutionRequest`] with the same `request_id`.
#[serde_with::serde_as]
#[derive(Debug, Deserialize, Serialize)]
struct ExecutionResponse {
    /// The request ID, allowing for differentiating multiple in-flight
    /// requests.
    request_id: Option<i32>,
    /// The data returned by the contract call.
    #[serde_as(as = "serde_with::hex::Hex")]
    data: Vec<u8>,
    /// A possible error happening during the contract call.
    error: Option<String>,
}

#[derive(Debug)]
enum ExecutionError {
    Http(hyper::http::Error),
    Hyper(hyper::Error),
    Json(serde_json::Error),
    Protocol(tungstenite::error::ProtocolError),
    Tungstenite(tungstenite::Error),
}

impl Display for ExecutionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionError::Http(err) => write!(f, "{err}"),
            ExecutionError::Hyper(err) => write!(f, "{err}"),
            ExecutionError::Json(err) => write!(f, "{err}"),
            ExecutionError::Protocol(err) => write!(f, "{err}"),
            ExecutionError::Tungstenite(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for ExecutionError {}

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

    use tokio::net::TcpStream;
    use tokio_tungstenite::client_async;

    /// A [`QueryRaw`] implementation that returns the same data it is passed.
    struct TestQueryRaw;

    impl QueryRaw for TestQueryRaw {
        type Error = Infallible;

        fn query_raw<N, C>(
            &self,
            _contract: ContractId,
            _fn_name: N,
            fn_args: C,
        ) -> Result<Vec<u8>, Self::Error>
        where
            N: AsRef<str>,
            C: Into<Vec<u8>>,
        {
            Ok(fn_args.into())
        }
    }

    #[tokio::test]
    async fn http_query() {
        let server = HttpServer::bind(TestQueryRaw, "localhost:0")
            .await
            .expect("Binding the server to the address should succeed");

        let fn_arg = Vec::from(&b"I am call data 0"[..]);

        let request = serde_json::to_vec(&ExecutionRequest {
            request_id: Some(2),
            contract: [42; 32],
            fn_name: "test_function".to_string(),
            fn_args: fn_arg.clone(),
        })
        .expect("Serializing request should succeed");

        let client = reqwest::Client::new();
        let response = client
            .post(format!("http://{}/", server.local_addr()))
            .body(Body::from(request))
            .send()
            .await
            .expect("Requesting should succeed");

        let response = serde_json::from_slice::<ExecutionResponse>(
            &response.bytes().await.expect("There should be a response"),
        )
        .expect("Response should deserialize successfully");

        assert_eq!(fn_arg, response.data, "Data received the same as sent");
    }

    #[tokio::test]
    async fn websocket_queries() {
        let server = HttpServer::bind(TestQueryRaw, "localhost:0")
            .await
            .expect("Binding the server to the address should succeed");

        let stream = TcpStream::connect(server.local_addr())
            .await
            .expect("Connecting to the server should succeed");

        let (mut stream, _) = client_async("ws://localhost", stream)
            .await
            .expect("Handshake with the server should succeed");

        let fn_args = [
            Vec::from(&b"I am call data 0"[..]),
            Vec::from(&b"I am call data 1"[..]),
            Vec::from(&b"I am call data 2"[..]),
            Vec::from(&b"I am call data 3"[..]),
        ];

        let requests = fn_args
            .iter()
            .cloned()
            .enumerate()
            .map(|(i, fn_args)| {
                serde_json::to_string(&ExecutionRequest {
                    request_id: Some(i as i32),
                    contract: [42; 32],
                    fn_name: "test_function".to_string(),
                    fn_args,
                })
                .expect("Serializing request should succeed")
            })
            .collect::<Vec<_>>();

        let request_num = requests.len();

        for request in requests {
            stream
                .send(Message::Text(request))
                .await
                .expect("Sending request to the server should succeed");
        }

        let mut responses =
            Vec::<ExecutionResponse>::with_capacity(request_num);

        while responses.len() < request_num {
            let msg = stream
                .next()
                .await
                .expect("Stream shouldn't close while awaiting responses")
                .expect("Response should be received without error");

            let msg = match msg {
                Message::Text(msg) => msg,
                _ => panic!("Shouldn't receive anything but text"),
            };

            let response = serde_json::from_str(&msg)
                .expect("Response should deserialize successfully");
            responses.push(response);
        }

        for response in responses {
            let expected_data = &fn_args[response
                .request_id
                .expect("Should have a request ID in the response")
                as usize];
            assert_eq!(
                &response.data, expected_data,
                "Response data should be the same as the request `fn_args`"
            );
            assert!(matches!(response.error, None), "There should be no error");
        }
    }
}
