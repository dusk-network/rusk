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

use node_data::message::payload::{GetData, InvParam, InvType};
use smallvec::SmallVec;
use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use node_data::message::{payload, AsyncQueue};
use node_data::message::{Payload, Topics};
use tokio::sync::{RwLock, Semaphore};
use tracing::{debug, info, warn};

const QUEUE_LIMIT: usize = 10_000;

const TOPICS: &[u8] = &[
    Topics::GetBlocks as u8,
    Topics::GetMempool as u8,
    Topics::GetInv as u8,
    Topics::GetData as u8,
    Topics::GetCandidate as u8,
];

struct Response {
    /// A response usually consists of a single message. However in case of
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
    requests: AsyncQueue<Message>,

    /// Limits the number of ongoing requests.
    limit_ongoing_requests: Arc<Semaphore>,

    conf: conf::Params,
}

impl DataBrokerSrv {
    pub fn new(conf: conf::Params) -> Self {
        info!("DataBrokerSrv::new with conf: {}", conf);
        let permits = conf.max_ongoing_requests;
        Self {
            conf,
            requests: AsyncQueue::bounded(QUEUE_LIMIT),
            limit_ongoing_requests: Arc::new(Semaphore::new(permits)),
        }
    }
}

#[async_trait]
impl<N: Network, DB: database::DB, VM: vm::VMExecution>
    LongLivedService<N, DB, VM> for DataBrokerSrv
{
    async fn initialize(
        &mut self,
        _network: Arc<RwLock<N>>,
        _db: Arc<RwLock<DB>>,
        _vm: Arc<RwLock<VM>>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

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
            self.requests.clone(),
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
            let msg = self.requests.recv().await?;

            let network = network.clone();
            let db = db.clone();
            let conf = self.conf.clone();

            // Spawn a task to handle the request asynchronously.
            tokio::spawn(async move {
                match Self::handle_request(&db, &msg, &conf).await {
                    Ok(resp) => {
                        // Send response
                        let net = network.read().await;
                        for msg in resp.msgs {
                            let send = net.send_to_peer(&msg, resp.recv_peer);
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
    async fn handle_request<DB: database::DB>(
        db: &Arc<RwLock<DB>>,
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

        match &msg.payload {
            // Handle GetCandidate requests
            Payload::GetCandidate(m) => {
                let msg = Self::handle_get_candidate(db, m).await?;
                Ok(Response::new_from_msg(msg, recv_peer))
            }
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
            // Handle GetInv requests
            Payload::GetInv(m) => {
                let msg = Self::handle_inv(db, m, conf.max_inv_entries).await?;
                Ok(Response::new_from_msg(msg, recv_peer))
            }
            // Handle GetData requests
            Payload::GetData(m) => {
                let msgs =
                    Self::handle_get_data(db, m, conf.max_inv_entries).await?;
                Ok(Response::new(msgs, recv_peer))
            }
            _ => Err(anyhow::anyhow!("unhandled message payload")),
        }
    }

    /// Handles GetCandidate requests.
    ///
    /// Message flow: GetCandidate -> CandidateResp
    async fn handle_get_candidate<DB: database::DB>(
        db: &Arc<RwLock<DB>>,
        m: &payload::GetCandidate,
    ) -> Result<Message> {
        let res = db
            .read()
            .await
            .view(|t| t.fetch_candidate_block(&m.hash))
            .map_err(|e| {
                anyhow::anyhow!("could not fetch candidate block: {:?}", e)
            })?;

        let block =
            res.ok_or_else(|| anyhow::anyhow!("could not find candidate"))?;

        Ok(Message::new_get_candidate_resp(payload::GetCandidateResp {
            candidate: block,
        }))
    }

    /// Handles GetMempool requests.
    /// Message flow: GetMempool -> Inv -> GetData -> Tx
    async fn handle_get_mempool<DB: database::DB>(
        db: &Arc<RwLock<DB>>,
    ) -> Result<Message> {
        let mut inv = payload::Inv::default();

        db.read()
            .await
            .view(|t| {
                for hash in t.get_txs_hashes()? {
                    inv.add_tx_hash(hash);
                }

                if inv.inv_list.is_empty() {
                    return Err(anyhow::anyhow!("mempool is empty"));
                }

                Ok(())
            })
            .map_err(|e| anyhow::anyhow!(e))?;

        Ok(Message::new_inv(inv))
    }

    /// Handles GetBlocks message request.
    ///
    ///  Message flow: GetBlocks -> Inv -> GetData -> Block
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

                loop {
                    locator += 1;
                    match t.fetch_block_hash_by_height(locator)? {
                        Some(bh) => {
                            inv.add_block_from_hash(bh);
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

        Ok(Message::new_inv(inv))
    }

    /// Handles inventory message request.
    ///
    /// This takes an inventory message (topics.Inv), checks it for any
    /// items that the node state is missing, puts these items in a GetData
    /// wire message, and sends it back to request the items in full.
    ///
    /// An item is a block or a transaction.
    async fn handle_inv<DB: database::DB>(
        db: &Arc<RwLock<DB>>,
        m: &node_data::message::payload::Inv,
        max_entries: usize,
    ) -> Result<Message> {
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
                    InvType::MempoolTx => {
                        if let InvParam::Hash(hash) = &i.param {
                            if Mempool::get_tx(&t, *hash)?.is_none() {
                                inv.add_tx_hash(*hash);
                            }
                        }
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

        Ok(Message::new_get_data(GetData { inner: inv }))
    }

    /// Handles GetData message request.
    ///
    /// The response to a GetData message is a vector of messages, each of which
    /// could be either topics.Block or topics.Tx.
    async fn handle_get_data<DB: database::DB>(
        db: &Arc<RwLock<DB>>,
        m: &node_data::message::payload::GetData,
        max_entries: usize,
    ) -> Result<Vec<Message>> {
        db.read().await.view(|t| {
            Ok(m.inner
                .inv_list
                .iter()
                .filter_map(|i| match i.inv_type {
                    InvType::BlockFromHeight => {
                        if let InvParam::Height(height) = &i.param {
                            Ledger::fetch_block_by_height(&t, *height)
                                .ok()
                                .flatten()
                                .map(Message::new_block)
                        } else {
                            None
                        }
                    }
                    InvType::BlockFromHash => {
                        if let InvParam::Hash(hash) = &i.param {
                            Ledger::fetch_block(&t, hash)
                                .ok()
                                .flatten()
                                .map(Message::new_block)
                        } else {
                            None
                        }
                    }
                    InvType::MempoolTx => {
                        if let InvParam::Hash(hash) = &i.param {
                            Mempool::get_tx(&t, *hash)
                                .ok()
                                .flatten()
                                .map(Message::new_transaction)
                        } else {
                            None
                        }
                    }
                })
                .take(max_entries)
                .collect())
        })
    }
}
