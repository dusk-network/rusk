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
use tokio::sync::broadcast::{self, error::RecvError, Sender};
use tonic::{Request, Response, Status};
use tracing::{debug, error, warn};

use crate::error::Error;

pub use super::rusk_proto::{
    network_server::{Network, NetworkServer},
    BroadcastMessage, Message, MessageMetadata, Null, PropagateMessage,
    SendMessage,
};
use futures::Stream;
use std::io::{ErrorKind, Write};
use std::{net::SocketAddr, pin::Pin};

use super::{TX_TYPE_TRANSFER, TX_VERSION};

pub struct KadcastDispatcher {
    dummy_addr: SocketAddr,
    peer: Peer,
    inbound_dispatcher: Sender<(Vec<u8>, SocketAddr, u8)>,
}

impl KadcastDispatcher {
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
        let req = request.get_ref();

        let tx = Transaction::from_slice(&req.message)
            .map_err(Error::Serialization)?;
        let wire_message = serialization::network_marshal(&tx)?;

        self.inbound_dispatcher
            .send((wire_message, self.dummy_addr, 0))
            .unwrap_or_else(|e| {
                warn!("Error in dispatcher notification {}", e);
                0
            });
        self.peer.broadcast(&req.message, None).await;
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

    // const MAGIC_MAINNET: u32 = 0x7630401f;
    // const MAGIC_TESTNET: u32 = 0x74746e41;
    const MAGIC_DEVNET: u32 = 0x74736e40;
    // const MAGIC_STRESSNET: u32 = 0x74726e39;

    const MAGIC_BYTES: [u8; 4] = MAGIC_DEVNET.to_le_bytes();
    const TX_CATEGORY: u8 = 10;

    const RESERVED_FIELDS_BYTES: [u8; 8] = [0; 8];
    const TX_HEADER_LEN: u64 = {
        4u64 + // MAGIC
        8u64 + // RESERVED_FIELDS
        4u64 // CHECKSUM
    };
    const DUMMY_HASH: [u8; 32] = [0; 32];

    /// This serialize a transaction in a way that is handled by the network
    pub(super) fn network_marshal(tx: &Transaction) -> Result<Vec<u8>> {
        // WIRE FORMAT:
        // - Length (uint64LE)
        // - Magic (4bytes)
        // - ReservedFields (8bytes) -- ex timestamp --> now int64=0
        // - Checksum - blake2b256(Transaction)
        // - Transaction

        let tx_wire = tx_marshal(tx)?;
        let checksum = {
            let mut hasher = Blake2b::<U32>::new();
            hasher.update(&tx_wire);
            hasher.finalize().to_vec()
        };
        let tx_len: u64 = tx_wire.len().try_into().map_err(|_| {
            std::io::Error::new(ErrorKind::Other, "Tx len too long!")
        })?;

        let message_len = TX_HEADER_LEN + tx_len;
        let mut network_message = &mut message_len.to_le_bytes()[..];
        network_message.write_all(&MAGIC_BYTES[..])?;
        network_message.write_all(&RESERVED_FIELDS_BYTES[..])?;
        network_message.write_all(&checksum)?;
        network_message.write_all(&tx_wire)?;
        Ok(network_message.to_vec())
    }

    fn tx_marshal(tx: &Transaction) -> Result<Vec<u8>> {
        // TX FORMAT
        // - Category
        // - Transaction
        //   - uint32LE version
        //   - uint32LE txType
        //   - Payload
        //     - uint32LE lenght
        //     - blob  payload
        //   - Hash (256 bit)
        //   - GasLimit (uint64LE)
        //   - GasPrice (uint64LE)
        let mut tx_wire = &mut [TX_CATEGORY][..];
        tx_wire.write_all(&TX_VERSION.to_le_bytes()[..])?;
        tx_wire.write_all(&TX_TYPE_TRANSFER.to_le_bytes()[..])?;

        let payload = tx.to_var_bytes();
        tx_wire.write_all(&(payload.len() as u32).to_le_bytes()[..])?;
        tx_wire.write_all(&payload[..])?;
        tx_wire.write_all(&DUMMY_HASH[..])?;
        tx_wire.write_all(&0u64.to_le_bytes()[..])?; //GasLimit
        tx_wire.write_all(&0u64.to_le_bytes()[..])?; //GasPrice
        Ok(tx_wire.to_vec())
    }
}
