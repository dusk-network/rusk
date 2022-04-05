// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Public Key infrastructure service implementation for the Rusk server.

use crate::Result;
use blake2::{digest::consts::U32, Blake2b, Digest};
use dusk_wallet_core::Transaction;
use kadcast::config::Config as KadcastConfig;
use kadcast::{MessageInfo, NetworkListen, Peer};
use tokio::sync::broadcast::Receiver;
use tokio::sync::broadcast::{self, error::RecvError, Sender};
use tonic::{Request, Response, Status};
use tracing::{debug, error, warn};

use crate::error::Error;

use futures::Stream;
pub use rusk_schema::{
    network_server::{Network, NetworkServer},
    AliveNodesRequest, AliveNodesResponse, BroadcastMessage, Message,
    MessageMetadata, Null, PropagateMessage, SendMessage,
};
use std::io::Write;
use std::{net::SocketAddr, pin::Pin};

use super::{TX_TYPE_TRANSFER, TX_VERSION};

pub struct KadcastDispatcher {
    dummy_addr: SocketAddr,
    peer: Peer,
    inbound_dispatcher: Sender<(Vec<u8>, SocketAddr, u8)>,
}

impl KadcastDispatcher {
    pub fn subscribe(&self) -> Receiver<(Vec<u8>, SocketAddr, u8)> {
        self.inbound_dispatcher.subscribe()
    }

    pub fn new(config: KadcastConfig, hash_message: bool) -> KadcastDispatcher {
        // Creating a broadcast channel which each grpc `listen` calls will
        // listen to.
        // The inbound_dispatcher is used by the KadcastListener to forward
        // the received messages.
        // The receiver is discarded because at the moment 0 there is no one
        // listening.
        // When a `listen` call is received, a new receiver is created using
        // `inbound_dispatcher.subscribe`
        let inbound_dispatcher = broadcast::channel(100).0;
        let listener = KadcastListener {
            inbound_dispatcher: inbound_dispatcher.clone(),
            hash_message,
        };

        KadcastDispatcher {
            peer: Peer::new(config, listener),
            inbound_dispatcher,
            dummy_addr: "127.0.0.1:1".parse().expect("Unable to parse address"),
        }
    }
}

impl Default for KadcastDispatcher {
    fn default() -> KadcastDispatcher {
        KadcastDispatcher::new(KadcastConfig::default(), false)
    }
}
struct KadcastListener {
    inbound_dispatcher: broadcast::Sender<(Vec<u8>, SocketAddr, u8)>,
    hash_message: bool,
}

impl NetworkListen for KadcastListener {
    fn on_message(&self, message: Vec<u8>, metadata: MessageInfo) {
        let mut message = message;
        if self.hash_message {
            let mut hasher = Blake2b::<U32>::new();
            hasher.update(message);
            message = hasher.finalize().to_vec();
        }
        self.inbound_dispatcher
            .send((message, metadata.src(), metadata.height()))
            .unwrap_or_else(|e| {
                warn!("Error in dispatcher notification {}", e);
                0
            });
    }
}

#[tonic::async_trait]
impl Network for KadcastDispatcher {
    async fn send(
        &self,
        request: Request<SendMessage>,
    ) -> Result<Response<Null>, Status> {
        debug!("Received SendMessage request");
        let req = request.get_ref();
        self.peer
            .send(
                &req.message,
                req.target_address.parse().map_err(|_| {
                    Status::invalid_argument("Unable to parse address")
                })?,
            )
            .await;
        Ok(Response::new(Null {}))
    }

    async fn propagate(
        &self,
        request: Request<PropagateMessage>,
    ) -> Result<Response<Null>, Status> {
        debug!("Received PropagateMessage request");

        let wire_message = {
            // Ensure that the received buffer is a transaction
            let verified_tx =
                Transaction::from_slice(&request.get_ref().message)
                    .map_err(Error::Serialization)?;
            let tx_bytes = verified_tx.to_var_bytes();
            serialization::network_marshal(&tx_bytes)?
        };

        self.peer.broadcast(&wire_message, None).await;
        self.inbound_dispatcher
            .send((wire_message, self.dummy_addr, 0))
            .unwrap_or_else(|e| {
                warn!("Error in dispatcher notification {}", e);
                0
            });
        Ok(Response::new(Null {}))
    }

    async fn broadcast(
        &self,
        request: Request<BroadcastMessage>,
    ) -> Result<Response<Null>, Status> {
        debug!("Received BroadcastMessage request");
        let req = request.get_ref();
        self.peer
            .broadcast(&req.message, Some(req.kadcast_height as usize))
            .await;
        Ok(Response::new(Null {}))
    }

    async fn alive_nodes(
        &self,
        request: Request<AliveNodesRequest>,
    ) -> Result<Response<AliveNodesResponse>, Status> {
        debug!("Received AliveNodes request");
        let max_nodes = request.get_ref().max_nodes as usize;
        let alive_nodes = self.peer.alive_nodes(max_nodes).await;
        let address = alive_nodes
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();
        Ok(Response::new(AliveNodesResponse { address }))
    }

    type ListenStream =
        Pin<Box<dyn Stream<Item = Result<Message, Status>> + Send + 'static>>;

    async fn listen(
        &self,
        _: Request<Null>,
    ) -> Result<Response<Self::ListenStream>, Status> {
        debug!("Received Listen request");
        let mut rx = self.inbound_dispatcher.subscribe();
        let output = async_stream::try_stream! {
            loop {
                match rx.recv().await {
                    Ok((message, source_address, k_height)) => {
                        yield Message {
                            message,
                            metadata: Some(MessageMetadata {
                                src_address: source_address.to_string(),
                                kadcast_height: k_height as u32,
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

mod serialization {
    use super::*;
    use dusk_bytes::Serializable;

    const TX_CATEGORY: u8 = 10;

    const RESERVED_FIELDS_LEN: usize = 8;
    const RESERVED_FIELDS_BYTES: [u8; RESERVED_FIELDS_LEN] =
        [0; RESERVED_FIELDS_LEN];

    const CHECKSUM_LEN: usize = 4;

    struct ProtocolVersion(u16, u16, u16);

    const VERSION: ProtocolVersion = ProtocolVersion(0, 1, 0);

    const TX_HEADER_LEN: usize = {
        core::mem::size_of::<ProtocolVersion>() + 2 + // VERSION + Reserved bytes
        RESERVED_FIELDS_LEN + // RESERVED_FIELDS
        CHECKSUM_LEN // CHECKSUM
    };

    /// This serialize a transaction in a way that is handled by the network
    pub(super) fn network_marshal(tx: &[u8]) -> Result<Vec<u8>> {
        // WIRE FORMAT:
        // - Length (uint64LE)
        // - Version (8bytes)
        // - ReservedFields (8bytes) -- ex timestamp --> now int64=0
        // - Checksum - blake2b256(Transaction)[..4]
        // - Transaction

        let tx_wire = tx_marshal(tx)?;
        let checksum = {
            let mut hasher = Blake2b::<U32>::new();
            hasher.update(&tx_wire);
            hasher.finalize()[..CHECKSUM_LEN].to_vec()
        };
        let tx_len = tx_wire.len();

        let message_len = TX_HEADER_LEN + tx_len;
        let mut network_message = vec![0u8; u64::SIZE + message_len];
        let mut buffer = &mut network_message[..];
        buffer.write_all(&(message_len as u64).to_le_bytes())?;
        buffer.write_all(&[0u8; 2])?; //Version reserved bytes
        buffer.write_all(&VERSION.0.to_le_bytes()[..])?;
        buffer.write_all(&VERSION.1.to_le_bytes()[..])?;
        buffer.write_all(&VERSION.2.to_le_bytes()[..])?;
        buffer.write_all(&RESERVED_FIELDS_BYTES[..])?;
        buffer.write_all(&checksum)?;
        buffer.write_all(&tx_wire)?;
        Ok(network_message)
    }

    fn tx_marshal(payload: &[u8]) -> Result<Vec<u8>> {
        // TX FORMAT
        // - Category
        // - Transaction
        //   - uint32LE version
        //   - uint32LE txType
        //   - Payload
        //     - uint32LE length
        //     - blob  payload
        let size = 1 // Category
            + u32::SIZE // Version
            + u32::SIZE // TxType
            + u32::SIZE // Payload Length
            + payload.len(); // Payload
        let mut tx_wire = vec![0u8; size];
        let mut buffer = &mut tx_wire[..];
        buffer.write_all(&[TX_CATEGORY])?;
        buffer.write_all(&TX_VERSION.to_le_bytes()[..])?;
        buffer.write_all(&TX_TYPE_TRANSFER.to_le_bytes()[..])?;
        buffer.write_all(&(payload.len() as u32).to_le_bytes()[..])?;
        buffer.write_all(payload)?;
        Ok(tx_wire)
    }
}
