// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![allow(unused)]

mod event;
pub(crate) use event::Request;

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

use futures_util::{SinkExt, StreamExt};

use crate::chain::RuskNode;
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
            let response = match request.target_type {
                0x02 if request.target == "chain" => {
                    sources.node.handle_request(request).await
                }
                _ => tungstenite::protocol::Message::text("Unsupported"),
            };
            stream.send(response).await;
        }

        let _ = stream
            .close(Some(CloseFrame {
                code: CloseCode::Normal,
                reason: Cow::from("Client closed"),
            }))
            .await;
    }
}
