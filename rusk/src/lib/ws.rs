// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![allow(unused)]

use std::borrow::Cow;
use std::convert::Infallible;
use std::sync::Arc;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, ToSocketAddrs};
use tokio::sync::mpsc;
use tokio::{io, task};

use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;

use futures_util::StreamExt;

use node::database;
use node::{Network, Node};

use crate::Rusk;

pub struct WsServer {
    shutdown: mpsc::Sender<Infallible>,
    handle: task::JoinHandle<()>,
}

impl WsServer {
    pub async fn bind<N: Network, DB: database::DB, A: ToSocketAddrs>(
        rusk: Rusk,
        node: Node<N, DB, Rusk>,
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

struct DataSources<N: Network, DB: database::DB> {
    rusk: Rusk,
    node: Node<N, DB, Rusk>,
}

async fn listening_loop<N: Network, DB: database::DB>(
    sources: DataSources<N, DB>,
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
#[derive(Debug)]
struct Request {
    headers: serde_json::Map<String, serde_json::Value>,
    target_type: u8,
    target: String,
    topic: String,
    data: Vec<u8>,
}

impl Request {
    fn parse(bytes: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
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
            data,
        })
    }
}

fn parse_len(
    bytes: &[u8],
) -> Result<(usize, &[u8]), Box<dyn std::error::Error>> {
    if bytes.len() < 4 {
        return Err("not enough bytes".to_string().into());
    }

    let len =
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    let (_, left) = bytes.split_at(len);

    Ok((len, left))
}

type Header<'a> = (serde_json::Map<String, serde_json::Value>, &'a [u8]);
fn parse_header(bytes: &[u8]) -> Result<Header, Box<dyn std::error::Error>> {
    let (len, bytes) = parse_len(bytes)?;
    if bytes.len() < len {
        return Err(format!("not enough bytes for parsed len {len}").into());
    }

    let (header_bytes, bytes) = bytes.split_at(len);
    let header = serde_json::from_slice(header_bytes)?;

    Ok((header, bytes))
}

fn parse_target_type(
    bytes: &[u8],
) -> Result<(u8, &[u8]), Box<dyn std::error::Error>> {
    if bytes.is_empty() {
        return Err("not enough bytes for target type".to_string().into());
    }

    let (target_type_bytes, bytes) = bytes.split_at(1);
    let target_type = target_type_bytes[0];

    Ok((target_type, bytes))
}

fn parse_string(
    bytes: &[u8],
) -> Result<(String, &[u8]), Box<dyn std::error::Error>> {
    let (len, bytes) = parse_len(bytes)?;
    if bytes.len() < len {
        return Err(format!("not enough bytes for parsed len {len}").into());
    }

    let (string_bytes, bytes) = bytes.split_at(len);
    let string = String::from_utf8(string_bytes.to_vec())?;

    Ok((string, bytes))
}

#[allow(unused)]
struct StateQuery {}
#[allow(unused)]
struct ChainQuery {}

async fn handle_stream<
    N: Network,
    DB: database::DB,
    S: AsyncRead + AsyncWrite + Unpin,
>(
    sources: Arc<DataSources<N, DB>>,
    stream: S,
) {
    if let Ok(mut stream) = accept_async(stream).await {
        while let Some(Ok(msg)) = stream.next().await {
            let data = msg.into_data();
            let r = Request::parse(&data);

            if r.is_err() {
                break;
            }
            let request = r.unwrap();
        }

        let _ = stream
            .close(Some(CloseFrame {
                code: CloseCode::Normal,
                reason: Cow::from("Client closed"),
            }))
            .await;
    }
}
