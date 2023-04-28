// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::database::{Candidate, Ledger, Mempool};
use crate::{database, vm, Network};
use crate::{LongLivedService, Message};
use anyhow::{anyhow, bail, Result};

use dusk_consensus::user::committee::CommitteeSet;
use smallvec::SmallVec;
use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use dusk_consensus::commons::{ConsensusError, Database, RoundUpdate};
use dusk_consensus::consensus::Consensus;
use dusk_consensus::contract_state::{
    CallParams, Error, Operations, Output, StateRoot,
};
use dusk_consensus::user::provisioners::Provisioners;
use node_data::ledger::{self, Block, Hash, Header};
use node_data::message::{payload, AsyncQueue, Metadata};
use node_data::message::{Payload, Topics};
use node_data::Serializable;
use tokio::sync::{oneshot, Mutex, RwLock, Semaphore};
use tokio::task::JoinHandle;

use std::any;

const MAX_ONGOING_REQUESTS: usize = 100;
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
    fn new_from_msg(msg: Message, recv_peer: SocketAddr) -> Self {
        Self {
            msgs: SmallVec::from_buf([msg]),
            recv_peer,
        }
    }
}

pub struct DataBrokerSrv {
    /// A queue of pending requests to process.
    /// Request here is literally a GET message
    requests: AsyncQueue<Message>,

    /// Limits the number of ongoing requests.
    limit_ongoing_requests: Arc<Semaphore>,
}

impl Default for DataBrokerSrv {
    fn default() -> Self {
        Self {
            requests: AsyncQueue::default(),
            limit_ongoing_requests: Arc::new(Semaphore::new(
                MAX_ONGOING_REQUESTS,
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
        vm: Arc<RwLock<VM>>,
    ) -> anyhow::Result<usize> {
        // Register routes
        LongLivedService::<N, DB, VM>::add_routes(
            self,
            TOPICS,
            self.requests.clone(),
            &network,
        )
        .await?;

        tracing::info!("data broker service started");

        loop {
            /// Wait until we can process a new request. We limit the number of
            /// concurrent requests to mitigate a DoS attack.
            let permit =
                self.limit_ongoing_requests.clone().acquire_owned().await?;

            // Wait for a request to process.
            let msg = self.requests.recv().await?;

            let network = network.clone();
            let db = db.clone();

            // Spawn a task to handle the request asynchronously.
            tokio::spawn(async move {
                match Self::handle_request(&network, &db, &msg).await {
                    Ok(resp) => {
                        // Send response
                        for msg in resp.msgs {
                            network
                                .read()
                                .await
                                .send_to_peer(&msg, resp.recv_peer)
                                .await;
                        }
                    }
                    Err(e) => {
                        tracing::warn!("error handling msg: {:?}", e);
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
        network: &Arc<RwLock<N>>,
        db: &Arc<RwLock<DB>>,
        msg: &Message,
    ) -> anyhow::Result<Response> {
        /// source address of the request becomes the receiver address of the
        /// response
        let recv_peer = msg
            .metadata
            .as_ref()
            .map(|m| m.src_addr)
            .ok_or_else(|| anyhow::anyhow!("invalid metadata src_addr"))?;

        match &msg.payload {
            // Handle GetCandidate requests
            Payload::GetCandidate(m) => {
                let msg =
                    Self::handle_get_candidate(network, db, m.clone()).await?;
                Ok(Response::new_from_msg(msg, recv_peer))
            }
            _ => Err(anyhow::anyhow!("unhandled message payload")),
        }
    }

    async fn handle_get_candidate<N: Network, DB: database::DB>(
        network: &Arc<RwLock<N>>,
        db: &Arc<RwLock<DB>>,
        m: node_data::message::payload::GetCandidate,
    ) -> Result<Message> {
        let mut res = Option::None;
        db.read()
            .await
            .view(|t| {
                res = t.fetch_candidate_block(&m.hash)?;
                Ok(())
            })
            .map_err(|e| {
                anyhow::anyhow!("could not fetch candidate block: {:?}", e)
            })?;

        let block =
            res.ok_or_else(|| anyhow::anyhow!("could not find block"))?;

        Ok(Message::new_candidate_resp(payload::CandidateResp {
            candidate: block,
        }))
    }
}
