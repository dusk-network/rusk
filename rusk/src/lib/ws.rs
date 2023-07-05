// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![allow(unused)]

use std::borrow::Cow;
use std::convert::Infallible;
use std::fmt::Formatter;
use std::sync::Arc;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, ToSocketAddrs};
use tokio::sync::{broadcast, mpsc};
use tokio::{io, task};

use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;

use futures_util::{SinkExt, StreamExt};
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use tokio_tungstenite::tungstenite::Message;

use node::database;
use node::{Network, Node};
use rusk_abi::ContractId;

use crate::Rusk;

/// This WebSocket server implements a very simple request-response protocol. It
/// waits for [`Request`]s and then returns one [`Response`], depending on the
/// result of the query to Rusk.
pub struct WsServer {
    shutdown: broadcast::Sender<Infallible>,
    handle: task::JoinHandle<()>,
}

impl WsServer {
    pub async fn bind<N: Network, DB: database::DB, A: ToSocketAddrs>(
        rusk: Rusk,
        node: Node<N, DB, Rusk>,
        addr: A,
    ) -> io::Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        let (shutdown_sender, shutdown_receiver) = broadcast::channel(1);

        let handle = task::spawn(listening_loop(
            DataSources { rusk, node },
            listener,
            shutdown_receiver,
        ));

        Ok(Self {
            shutdown: shutdown_sender,
            handle,
        })
    }
}

struct DataSources<N: Network, DB: database::DB> {
    rusk: Rusk,
    node: Node<N, DB, Rusk>,
}

/// Accepts incoming streams and passes them to [`handle_stream`], together with
/// the given data sources.
///
/// When all [`mpsc::Sender`]s that are coupled to the `shutdown` receiver are
/// dropped, the loop will exit and all streams will be closed.
async fn listening_loop<N: Network, DB: database::DB>(
    sources: DataSources<N, DB>,
    listener: TcpListener,
    mut shutdown: broadcast::Receiver<Infallible>,
) {
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
async fn handle_stream<
    N: Network,
    DB: database::DB,
    S: AsyncRead + AsyncWrite + Unpin,
>(
    sources: Arc<DataSources<N, DB>>,
    stream: S,
    mut shutdown: broadcast::Receiver<Infallible>,
) {
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
                        Some(msg) => match msg {
                            Ok(msg) => match msg {
                                // We received a request and should spawn a new tas to handle it.
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
                            Err(err) => {
                                let _ = stream.close(Some(CloseFrame {
                                    code: CloseCode::Error,
                                    reason: Cow::from(format!("Failed serializing response: {err}")),
                                })).await;
                                break;
                            }
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

async fn handle_request<N: Network, DB: database::DB>(
    sources: Arc<DataSources<N, DB>>,
    request: Request,
    responder: mpsc::UnboundedSender<Response>,
) {
    let rusk = &sources.rusk;

    let rsp = match rusk.query_raw(
        request.contract.0,
        request.fn_name,
        request.fn_args,
    ) {
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

/// A request sent by the websocket client, asking for a specific contract
/// function to be executed with the given arguments.
#[derive(Debug, Deserialize, Serialize)]
struct Request {
    /// The request ID, allowing for differentiating multiple in-flight
    /// requests.
    request_id: i32,
    /// The contract to call.
    contract: WrappedContractId,
    /// The function name to call in the contract.
    fn_name: String,
    /// The arguments to pass to the function.
    fn_args: Vec<u8>,
}

/// Response to a [`Request`] with the same `request_id`.
#[derive(Debug, Deserialize, Serialize)]
struct Response {
    /// The request ID, allowing for differentiating multiple in-flight
    /// requests.
    request_id: i32,
    /// The data returned by the contract call.
    data: Vec<u8>,
    /// A possible error happening during the contract call.
    error: Option<String>,
}

#[derive(Debug)]
struct WrappedContractId(ContractId);

impl Serialize for WrappedContractId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(self.0.as_bytes())
    }
}

impl<'de> Deserialize<'de> for WrappedContractId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct WrappedContractIdVisitor;
        impl<'de> Visitor<'de> for WrappedContractIdVisitor {
            type Value = WrappedContractId;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                write!(formatter, "32 bytes")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if v.len() != 32 {
                    return Err(de::Error::invalid_length(v.len(), &Self));
                }

                let mut bytes = [0u8; 32];
                bytes.copy_from_slice(v);

                Ok(WrappedContractId(ContractId::from_bytes(bytes)))
            }
        }

        deserializer.deserialize_bytes(WrappedContractIdVisitor)
    }
}
