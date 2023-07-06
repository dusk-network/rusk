// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::borrow::Cow;
use std::convert::Infallible;
use std::fmt::Display;
use std::net::SocketAddr;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, ToSocketAddrs};
use tokio::sync::{broadcast, mpsc};
use tokio::{io, task};

use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;
use tokio_tungstenite::tungstenite::Message;

use rusk_abi::ContractId;
use serde::{Deserialize, Serialize};

/// This WebSocket server implements a very simple request-response protocol. It
/// waits for [`Request`]s and then returns one [`Response`], depending on the
/// result of the query to Rusk.
pub struct WsServer {
    handle: task::JoinHandle<()>,
    local_addr: SocketAddr,
    _shutdown: broadcast::Sender<Infallible>,
}

impl WsServer {
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

/// Accepts incoming streams and passes them to [`handle_stream`], together with
/// the given data sources.
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

                task::spawn(handle_stream(sources.clone(), stream, shutdown.resubscribe()));
            }
        }
    }
}

/// Receives a stream, performs the WebSocket handshake on it, and keeps
/// receiving messages until either the stream closes, or the server shuts down.
///
/// If a message is received that the server cannot parse, then the stream will
/// be closed.
async fn handle_stream<VM, S>(
    sources: Arc<DataSources<VM>>,
    stream: S,
    mut shutdown: broadcast::Receiver<Infallible>,
) where
    VM: QueryRaw,
    S: AsyncRead + AsyncWrite + Unpin,
{
    let (responder, mut responses) = mpsc::unbounded_channel::<Response>();

    if let Ok(mut stream) = accept_async(stream).await {
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
                        serde_json::to_string(&Response {
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
                                        task::spawn(handle_request(
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
}

async fn handle_request<VM>(
    sources: Arc<DataSources<VM>>,
    request: Request,
    responder: mpsc::UnboundedSender<Response>,
) where
    VM: QueryRaw,
{
    let vm = &sources.vm;

    let contract = ContractId::from_bytes(request.contract);

    let rsp = match vm.query_raw(contract, request.fn_name, request.fn_args) {
        Ok(data) => Response {
            request_id: request.request_id,
            data,
            error: None,
        },
        Err(err) => Response {
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

/// A request sent by the websocket client, asking for a specific contract
/// function to be executed with the given arguments.
#[serde_with::serde_as]
#[derive(Debug, Deserialize, Serialize)]
struct Request {
    /// The request ID, allowing for differentiating multiple in-flight
    /// requests.
    request_id: i32,
    /// The contract to call.
    #[serde_as(as = "serde_with::hex::Hex")]
    contract: [u8; 32],
    /// The function name to call in the contract.
    fn_name: String,
    /// The arguments to pass to the function.
    #[serde_as(as = "serde_with::hex::Hex")]
    fn_args: Vec<u8>,
}

/// Response to a [`Request`] with the same `request_id`.
#[serde_with::serde_as]
#[derive(Debug, Deserialize, Serialize)]
struct Response {
    /// The request ID, allowing for differentiating multiple in-flight
    /// requests.
    request_id: i32,
    /// The data returned by the contract call.
    #[serde_as(as = "serde_with::hex::Hex")]
    data: Vec<u8>,
    /// A possible error happening during the contract call.
    error: Option<String>,
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
    async fn multiple_queries() {
        let server = WsServer::bind(TestQueryRaw, "localhost:0")
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
                serde_json::to_string(&Request {
                    request_id: i as i32,
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

        let mut responses = Vec::<Response>::with_capacity(request_num);

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
            let expected_data = &fn_args[response.request_id as usize];
            assert_eq!(
                &response.data, expected_data,
                "Response data should be the same as the request `fn_args`"
            );
            assert!(matches!(response.error, None), "There should be no error");
        }
    }
}
