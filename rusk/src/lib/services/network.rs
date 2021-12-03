// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Public Key infrastructure service implementation for the Rusk server.

use kadcast::{MessageInfo, NetworkListen, Peer};
use tokio::sync::broadcast::{self, error::RecvError, Sender};
use tonic::{Request, Response, Status};
use tracing::{error, info, warn};

pub use super::rusk_proto::{
    network_server::{Network, NetworkServer},
    BroadcastMessage, Message, MessageMetadata, Null, SendMessage,
};
use futures::Stream;
use std::{net::SocketAddr, pin::Pin};

pub struct RuskNetwork {
    peer: Peer,
    sender: Sender<(Vec<u8>, SocketAddr, u8)>,
}

impl Default for RuskNetwork {
    fn default() -> RuskNetwork {
        let grpc_sender = broadcast::channel(100).0;
        let listener = KadcastListener {
            grpc_sender: grpc_sender.clone(),
        };

        // TODO: this should be instantiate with the correct config
        RuskNetwork {
            peer: Peer::builder("127.0.0.1:9999".to_string(), vec![], listener)
                .build(),
            sender: grpc_sender,
        }
    }
}
struct KadcastListener {
    grpc_sender: broadcast::Sender<(Vec<u8>, SocketAddr, u8)>,
}

impl NetworkListen for KadcastListener {
    fn on_message(&self, message: Vec<u8>, metadata: MessageInfo) {
        self.grpc_sender
            .send((message, metadata.src(), metadata.height()))
            .unwrap_or_else(|e| {
                println!("Error {}", e);
                0
            });
    }
}

#[tonic::async_trait]
impl Network for RuskNetwork {
    async fn send(
        &self,
        request: Request<SendMessage>,
    ) -> Result<Response<Null>, Status> {
        info!("Recieved SendMessage request");
        self.peer
            .send(
                &request.get_ref().message,
                request.get_ref().target_address.parse().map_err(|_| {
                    Status::invalid_argument("Unable to parse address")
                })?,
            )
            .await;
        Ok(Response::new(Null {}))
    }

    async fn broadcast(
        &self,
        request: Request<BroadcastMessage>,
    ) -> Result<Response<Null>, Status> {
        info!("Recieved BroadcastMessage request");
        self.peer
            .broadcast(
                &request.get_ref().message,
                request.get_ref().kadcast_height.map(|h| h as usize),
            )
            .await;
        Ok(Response::new(Null {}))
    }

    type ListenStream =
        Pin<Box<dyn Stream<Item = Result<Message, Status>> + Send + 'static>>;

    async fn listen(
        &self,
        _: Request<Null>,
    ) -> Result<Response<Self::ListenStream>, Status> {
        info!("Recieved Listen request");
        let mut rx = self.sender.subscribe();
        let output = async_stream::try_stream! {
            loop {
                match rx.recv().await {
                    Ok(ev) => {
                        yield Message {
                            message: ev.0,
                            metadata: Some(MessageMetadata {
                                src_address: ev.1.to_string(),
                                kadcast_height: ev.2 as u32,

                            }),
                        }
                    }
                    Err(e) => match e {
                        RecvError::Closed => {
                            error!("Sender stream is closed");
                            return;
                        },
                        RecvError::Lagged(skipped) => warn!("Skipped {} message", skipped)
                    }
                }
            }
        };
        Ok(Response::new(Box::pin(output) as Self::ListenStream))
    }
}
