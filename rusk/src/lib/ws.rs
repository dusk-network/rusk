// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::convert::Infallible;
use std::sync::Arc;

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, ToSocketAddrs};
use tokio::sync::mpsc;
use tokio::{io, task};

use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::CloseFrame;

use node::database;
use node::{Network, Node};

use crate::Rusk;

pub struct WsServer {
    _shutdown: mpsc::Sender<Infallible>,
    _handle: task::JoinHandle<()>,
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
            DataSources {
                _rusk: rusk,
                _node: node,
            },
            listener,
            shutdown_receiver,
        ));

        Ok(Self {
            _shutdown: shutdown_sender,
            _handle: handle,
        })
    }
}

struct DataSources<N: Network, DB: database::DB> {
    _rusk: Rusk,
    _node: Node<N, DB, Rusk>,
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

async fn handle_stream<
    N: Network,
    DB: database::DB,
    S: AsyncRead + AsyncWrite + Unpin,
>(
    _sources: Arc<DataSources<N, DB>>,
    stream: S,
) {
    if let Ok(mut stream) = accept_async(stream).await {
        let _ = stream
            .close(Some(CloseFrame {
                code: CloseCode::Unsupported,
                reason: "Unimplemented".into(),
            }))
            .await;
    }
}
