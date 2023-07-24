// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![allow(unused)]

use std::borrow::Cow;
use std::convert::Infallible;
use std::sync::Arc;

use node::database::rocksdb::Backend;
use serde::Deserialize;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, ToSocketAddrs};
use tokio::sync::mpsc;
use tokio::{io, task};

use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;

use futures_util::{SinkExt, StreamExt};

use node::database;
use node::{Network, Node};

use crate::chain::RuskNode;
use crate::graphql::DbContext;
use crate::Rusk;

pub struct WsServer {
    shutdown: mpsc::Sender<Infallible>,
    handle: task::JoinHandle<()>,
}

impl WsServer {
    pub async fn bind<A: ToSocketAddrs>(
        rusk: Rusk,
        node: RuskNode,
        addr: A,
    ) -> io::Result<Self> {
        let listener = TcpListener::bind(addr).await?;
        let (shutdown_sender, shutdown_receiver) = mpsc::channel(1);

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

struct DataSources {
    rusk: Rusk,
    node: RuskNode,
}

async fn listening_loop(
    sources: DataSources,
    listener: TcpListener,
    mut shutdown: mpsc::Receiver<Infallible>,
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

                task::spawn(handle_stream(sources.clone(), stream));
            }
        }
    }
}

/// A request sent by the websocket client.
#[derive(Debug, Deserialize)]
struct Request {
    headers: serde_json::Map<String, serde_json::Value>,
    target_type: u8,
    target: String,
    topic: String,
    #[serde(skip)]
    binary_data: Vec<u8>,
    data: String,
}

impl Request {
    fn parse(bytes: &[u8]) -> anyhow::Result<Self> {
        let a: Vec<u8> = vec![];
        let (headers, bytes) = parse_header(bytes)?;
        let (target_type, bytes) = parse_target_type(bytes)?;
        let (target, bytes) = parse_string(bytes)?;
        let (topic, bytes) = parse_string(bytes)?;
        let data = bytes.to_vec();
        Ok(Self {
            headers,
            target_type,
            target,
            topic,
            binary_data: data,
            data: "".into(),
        })
    }
}

fn parse_len(bytes: &[u8]) -> anyhow::Result<(usize, &[u8])> {
    if bytes.len() < 4 {
        return Err(anyhow::anyhow!("not enough bytes"));
    }

    let len =
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    let (_, left) = bytes.split_at(len);

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

#[allow(unused)]
struct StateQuery {}
#[allow(unused)]
struct ChainQuery {}

use crate::graphql::Query;
use juniper::EmptyMutation;
use juniper::EmptySubscription;
use juniper::OperationType;
use juniper::Variables;
type Schema = juniper::RootNode<
    'static,
    Query,
    EmptyMutation<DbContext>,
    EmptySubscription<DbContext>,
>;

async fn handle_stream<S: AsyncRead + AsyncWrite + Unpin>(
    sources: Arc<DataSources>,
    stream: S,
) {
    if let Ok(mut stream) = accept_async(stream).await {
        while let Some(Ok(msg)) = stream.next().await {
            let r = match msg.is_binary() {
                true => Request::parse(&msg.into_data())
                    .map_err(|e| anyhow::anyhow!(e)),
                false => serde_json::from_slice(&msg.into_data()[..])
                    .map_err(|e| anyhow::anyhow!(e)),
            };
            if r.is_err() {
                println!("{r:?}");
                break;
            }
            let request: Request = r.unwrap();
            match request.target_type {
                0x02 if request.target == "chain" => {
                    let ctx = DbContext(sources.node.db());

                    // // Run the executor.
                    match juniper::execute(
                        &request.data,
                        None,
                        &Schema::new(
                            Query,
                            EmptyMutation::new(),
                            EmptySubscription::new(),
                        ),
                        &Variables::new(),
                        &ctx,
                    )
                    .await
                    {
                        Err(e) => {
                            stream
                                .send(tungstenite::protocol::Message::text(
                                    format!("Error {e}"),
                                ))
                                .await;
                        }
                        Ok((res, _errors)) => {
                            stream
                                .send(tungstenite::protocol::Message::text(
                                    format!("{res}"),
                                ))
                                .await;
                        }
                    }
                }
                _ => {
                    stream
                        .send(tungstenite::protocol::Message::text(
                            "Unsupported",
                        ))
                        .await;
                }
            }
        }

        let _ = stream
            .close(Some(CloseFrame {
                code: CloseCode::Normal,
                reason: Cow::from("Client closed"),
            }))
            .await;
    }
}
