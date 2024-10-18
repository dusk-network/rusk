// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod conf;

use crate::database::{Candidate, Ledger, Mempool};
use crate::{database, vm, Network};
use crate::{LongLivedService, Message};
use anyhow::{anyhow, Result};
use std::cmp::min;

use node_data::message::payload::{self, GetResource, InvParam, InvType};
use node_data::message::{AsyncQueue, Payload, Topics};
use smallvec::SmallVec;
use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{RwLock, Semaphore};
use tracing::{debug, info, warn};

const TOPICS: &[u8] = &[
    Topics::GetBlocks as u8,
    Topics::GetMempool as u8,
    Topics::Inv as u8,
    Topics::GetResource as u8,
];

struct Response {
    /// A response usually consists of a single message. However, in case of
    /// GetMempool and GetBlocks we may need to send multiple messages in
    /// response to a single request.
    msgs: SmallVec<[Message; 1]>,

    /// Destination address of the response.
    recv_peer: SocketAddr,
}

impl Response {
    fn new(msgs: Vec<Message>, recv_peer: SocketAddr) -> Self {
        Self {
            msgs: SmallVec::from_vec(msgs),
            recv_peer,
        }
    }

    /// Creates a new response from a single message.
    fn new_from_msg(msg: Message, recv_peer: SocketAddr) -> Self {
        Self {
            msgs: SmallVec::from_buf([msg]),
            recv_peer,
        }
    }
}
/// Implements a request-for-data service.
///
/// The data broker acts as an intermediary between data producers (such as
/// Ledger, Candidates and Mempool databases ) and data consumers which could be
/// any node in the network that needs to recover any state.
///
/// Similar to a HTTP Server, the DataBroker service processes each request in
/// a separate tokio::task.
///
/// It also limits the number of concurrent requests.
pub struct DataBrokerSrv {
    /// A queue of pending requests to process.
    /// Request here is literally a GET message
    inbound: AsyncQueue<Message>,

    /// Limits the number of ongoing requests.
    limit_ongoing_requests: Arc<Semaphore>,

    conf: conf::Params,
}

impl DataBrokerSrv {
    pub fn new(conf: conf::Params) -> Self {
        info!("DataBrokerSrv::new with conf: {conf:?}");
        Self {
            conf,
            inbound: AsyncQueue::bounded(
                conf.max_queue_size,
                "databroker_inbound",
            ),
            limit_ongoing_requests: Arc::new(Semaphore::new(
                conf.max_ongoing_requests,
            )),
        }
    }
}

#[async_trait]
impl<N: Network, DB: database::DB, VM: vm::VMExecution>
    LongLivedService<N, DB, VM> for DataBrokerSrv
{
    async fn execute(
        &mut self,
        network: Arc<RwLock<N>>,
        db: Arc<RwLock<DB>>,
        _vm: Arc<RwLock<VM>>,
    ) -> anyhow::Result<usize> {
        if self.conf.max_ongoing_requests == 0 {
            return Err(anyhow!("max_ongoing_requests must be greater than 0"));
        }

        // Register routes
        LongLivedService::<N, DB, VM>::add_routes(
            self,
            TOPICS,
            self.inbound.clone(),
            &network,
        )
        .await?;

        info!("data_broker service started");

        loop {
            // Wait until we can process a new request. We limit the number of
            // concurrent requests to mitigate a DoS attack.
            let permit =
                self.limit_ongoing_requests.clone().acquire_owned().await?;

            // Wait for a request to process.
            let msg = self.inbound.recv().await?;

            let network = network.clone();
            let db = db.clone();
            let conf = self.conf;

            // Spawn a task to handle the request asynchronously.
            tokio::spawn(async move {
                match Self::handle_request::<N, DB>(&db, &network, &msg, &conf)
                    .await
                {
                    Ok(resp) => {
                        // Send response
                        let net = network.read().await;
                        for msg in resp.msgs {
                            let send = net.send_to_peer(msg, resp.recv_peer);
                            if let Err(e) = send.await {
                                warn!("Unable to send_to_peer {e}")
                            };

                            // Mitigate pressure on UDP buffers.
                            // Needed only in localnet.
                            if let Some(milli_sec) = conf.delay_on_resp_msg {
                                tokio::time::sleep(
                                    std::time::Duration::from_millis(milli_sec),
                                )
                                .await;
                            }
                        }
                    }
                    Err(e) => {
                        warn!("error on handling msg: {}", e);
                    }
                };

                // Release the permit.
                drop(permit);
            });
        }
    }

    /// Returns service name.
    fn name(&self) -> &'static str {
        "data_broker"
    }
}

impl DataBrokerSrv {
    /// Handles inbound messages.
    async fn handle_request<N: Network, DB: database::DB>(
        db: &Arc<RwLock<DB>>,
        network: &Arc<RwLock<N>>,
        msg: &Message,
        conf: &conf::Params,
    ) -> anyhow::Result<Response> {
        // source address of the request becomes the receiver address of the
        // response
        let recv_peer = msg
            .metadata
            .as_ref()
            .map(|m| m.src_addr)
            .ok_or_else(|| anyhow::anyhow!("invalid metadata src_addr"))?;

        debug!(event = "handle_request", ?msg);
        let this_peer = *network.read().await.public_addr();

        match &msg.payload {
            // Handle GetBlocks requests
            Payload::GetBlocks(m) => {
                let msg = Self::handle_get_blocks(db, m, conf.max_inv_entries)
                    .await?;
                Ok(Response::new_from_msg(msg, recv_peer))
            }
            // Handle GetMempool requests
            Payload::GetMempool(_) => {
                let msg = Self::handle_get_mempool(db).await?;
                Ok(Response::new_from_msg(msg, recv_peer))
            }
            // Handle Inv messages
            Payload::Inv(m) => {
                let msg =
                    Self::handle_inv(db, m, conf.max_inv_entries, this_peer)
                        .await?;
                Ok(Response::new_from_msg(msg, recv_peer))
            }
            // Handle GetResource requests
            Payload::GetResource(m) => {
                if m.is_expired() {
                    return Err(anyhow!("message has expired"));
                }

                match Self::handle_get_resource(db, m, conf.max_inv_entries)
                    .await
                {
                    Ok(msg_list) => {
                        Ok(Response::new(msg_list, m.get_addr().unwrap()))
                    }
                    Err(err) => {
                        // resource is not found, rebroadcast the request only
                        // if hops_limit is not reached
                        if let Some(m) = m.clone_with_hop_decrement() {
                            // Construct a new message with same
                            // Message::metadata but with decremented
                            // hops_limit
                            let mut msg = msg.clone();
                            msg.payload = Payload::GetResource(m);

                            debug!("resend a flood request {:?}", msg);

                            let _ = network
                                .read()
                                .await
                                .send_to_alive_peers(msg, 1)
                                .await;
                        }
                        Err(err)
                    }
                }
            }
            _ => Err(anyhow::anyhow!("unhandled message payload")),
        }
    }

    /// Handles GetMempool requests.
    /// Message flow: GetMempool -> Inv -> GetResource -> Tx
    async fn handle_get_mempool<DB: database::DB>(
        db: &Arc<RwLock<DB>>,
    ) -> Result<Message> {
        let mut inv = payload::Inv::default();

        db.read()
            .await
            .view(|t| {
                for hash in t.get_txs_ids()? {
                    inv.add_tx_id(hash);
                }

                if inv.inv_list.is_empty() {
                    return Err(anyhow::anyhow!("mempool is empty"));
                }

                Ok(())
            })
            .map_err(|e| anyhow::anyhow!(e))?;

        Ok(inv.into())
    }

    /// Handles GetBlocks message request.
    ///
    ///  Message flow: GetBlocks -> Inv -> GetResource -> Block
    async fn handle_get_blocks<DB: database::DB>(
        db: &Arc<RwLock<DB>>,
        m: &payload::GetBlocks,
        max_entries: usize,
    ) -> Result<Message> {
        let mut inv = payload::Inv::default();
        db.read()
            .await
            .view(|t| {
                let mut locator = t
                    .fetch_block(&m.locator)?
                    .ok_or_else(|| {
                        anyhow::anyhow!("could not find locator block")
                    })?
                    .header()
                    .height;

                let mut prev_block_hash = m.locator;

                loop {
                    locator += 1;
                    match t.fetch_block_hash_by_height(locator)? {
                        Some(bh) => {
                            let header =
                                t.fetch_block_header(&bh)?.ok_or_else(
                                    || anyhow!("block header not found"),
                                )?;

                            if header.prev_block_hash != prev_block_hash {
                                return Err(anyhow::anyhow!(
                                    "inconsistent chain"
                                ));
                            }

                            inv.add_block_from_hash(bh);
                            prev_block_hash = bh;
                        }
                        None => {
                            break;
                        }
                    }

                    //limit to the number of blocks to fetch
                    if inv.inv_list.len() >= max_entries {
                        break;
                    }
                }

                if inv.inv_list.is_empty() {
                    return Err(anyhow::anyhow!("no blocks found"));
                }

                Ok(())
            })
            .map_err(|e| anyhow::anyhow!(e))?;

        Ok(inv.into())
    }

    /// Handles inventory message request.
    ///
    /// This takes an inventory message (topics.Inv), checks it for any
    /// items that the node state is missing, puts these items in a GetResource
    /// wire message, and sends it back to request the items in full.
    ///
    /// An item is a block, a transaction, or a ValidationResult.
    async fn handle_inv<DB: database::DB>(
        db: &Arc<RwLock<DB>>,
        m: &node_data::message::payload::Inv,
        max_entries: usize,
        requester_addr: SocketAddr,
    ) -> Result<Message> {
        let mut max_entries = max_entries;
        if m.max_entries > 0 {
            max_entries = min(max_entries, m.max_entries as usize);
        }

        let inv = db.read().await.view(|t| {
            let mut inv = payload::Inv::default();
            for i in &m.inv_list {
                debug!(event = "handle_inv", ?i);
                match i.inv_type {
                    InvType::BlockFromHeight => {
                        if let InvParam::Height(height) = &i.param {
                            if Ledger::fetch_block_by_height(&t, *height)?
                                .is_none()
                            {
                                inv.add_block_from_height(*height);
                            }
                        }
                    }
                    InvType::BlockFromHash => {
                        if let InvParam::Hash(hash) = &i.param {
                            if Ledger::fetch_block(&t, hash)?.is_none() {
                                inv.add_block_from_hash(*hash);
                            }
                        }
                    }
                    InvType::CandidateFromHash => {
                        if let InvParam::Hash(hash) = &i.param {
                            if Candidate::fetch_candidate_block(&t, hash)?
                                .is_none()
                            {
                                inv.add_candidate_from_hash(*hash);
                            }
                        }
                    }
                    InvType::MempoolTx => {
                        if let InvParam::Hash(tx_id) = &i.param {
                            if Mempool::get_tx(&t, *tx_id)?.is_none() {
                                inv.add_tx_id(*tx_id);
                            }
                        }
                    }
                    InvType::CandidateFromIteration => {
                        if let InvParam::Iteration(ch) = &i.param {
                            if Candidate::fetch_candidate_block_by_iteration(
                                &t, ch,
                            )?
                            .is_none()
                            {
                                inv.add_candidate_from_iteration(*ch);
                            }
                        }
                    }
                    InvType::ValidationResult => {
                        //TODO: try fetch and ask if is missing
                    }
                }

                if inv.inv_list.len() >= max_entries {
                    break;
                }
            }

            Ok::<payload::Inv, anyhow::Error>(inv)
        })?;

        if inv.inv_list.is_empty() {
            return Err(anyhow::anyhow!("no items to fetch"));
        }

        // Send GetResource request with disabled rebroadcast (hops_limit = 1),
        // Inv message is part of one-to-one messaging flows
        // (GetBlocks/Mempool) so it should not be treated as flooding request
        Ok(GetResource::new(inv, Some(requester_addr), u64::MAX, 1).into())
    }

    /// Handles GetResource message request.
    ///
    /// The response to a GetResource message is a vector of messages, each of
    /// which could be either topics.Block or topics.Tx.
    async fn handle_get_resource<DB: database::DB>(
        db: &Arc<RwLock<DB>>,
        m: &node_data::message::payload::GetResource,
        max_entries: usize,
    ) -> Result<Vec<Message>> {
        let mut max_entries = max_entries;
        if m.get_inv().max_entries > 0 {
            max_entries = min(max_entries, m.get_inv().max_entries as usize);
        }

        db.read().await.view(|t| {
            let res: Vec<Message> = m
                .get_inv()
                .inv_list
                .iter()
                .filter_map(|i| match i.inv_type {
                    InvType::BlockFromHeight => {
                        if let InvParam::Height(height) = &i.param {
                            Ledger::fetch_block_by_height(&t, *height)
                                .ok()
                                .flatten()
                                .map(Message::from)
                        } else {
                            None
                        }
                    }
                    InvType::BlockFromHash => {
                        if let InvParam::Hash(hash) = &i.param {
                            Ledger::fetch_block(&t, hash)
                                .ok()
                                .flatten()
                                .map(Message::from)
                        } else {
                            None
                        }
                    }
                    InvType::CandidateFromHash => {
                        if let InvParam::Hash(hash) = &i.param {
                            Ledger::fetch_block(&t, hash)
                                .ok()
                                .flatten()
                                .or_else(|| {
                                    Candidate::fetch_candidate_block(&t, hash)
                                        .ok()
                                        .flatten()
                                })
                                .map(Message::from)
                        } else {
                            None
                        }
                    }
                    InvType::MempoolTx => {
                        if let InvParam::Hash(tx_id) = &i.param {
                            Mempool::get_tx(&t, *tx_id)
                                .ok()
                                .flatten()
                                .map(Message::from)
                        } else {
                            None
                        }
                    }
                    InvType::CandidateFromIteration => {
                        if let InvParam::Iteration(ch) = &i.param {
                            Candidate::fetch_candidate_block_by_iteration(
                                &t, ch,
                            )
                            .ok()
                            .flatten()
                            .map(|candidate| {
                                Message::from(payload::Candidate { candidate })
                            })
                        } else {
                            None
                        }
                    }
                    InvType::ValidationResult => {
                        //TODO: fetch and return ValidationQuorum
                        None
                    }
                })
                .take(max_entries)
                .collect();

            if res.is_empty() {
                // If nothing was found, return an error so that the caller is
                // instructed to rebroadcast the request, if needed
                debug!("handle_get_resource not found {:?}", m);
                return Err(anyhow!("not found"));
            }

            Ok(res)
        })
    }
}
