// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![allow(unused)]

mod request;

use std::borrow::Cow;
use std::convert::Infallible;
use std::sync::Arc;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, ToSocketAddrs};
use tokio::sync::mpsc;
use tokio::{io, stream, task};

use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;

use futures_util::StreamExt;

use node::database;
use node::{Network, Node};

use crate::ws::request::Request;
use crate::Rusk;

type Header<'a> = (serde_json::Map<String, serde_json::Value>, &'a [u8]);

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

        tracing::info!("Binded to {:?}", listener.local_addr());

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
                tracing::info!("Incoming ws connection");

                match r {
                    Ok((stream, addr)) => {
                        let sources = Arc::clone(&sources);

                        task::spawn(async move {
                            handle_stream(sources.clone(), stream).await;
                        });
                    },
                    Err(e) => {
                        tracing::error!("Error accepting connection: {:?}", e);

                        continue;
                    }
                }
            }
        }
    }
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

        tracing::info!("Client disconnected");

        let _ = stream
            .close(Some(CloseFrame {
                code: CloseCode::Normal,
                reason: Cow::from("Client closed"),
            }))
            .await;
    }
}
