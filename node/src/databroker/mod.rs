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
use tokio::sync::{oneshot, Mutex, RwLock};
use tokio::task::JoinHandle;

use std::any;

const TOPICS: &[u8] = &[
    Topics::GetBlocks as u8,
    Topics::GetData as u8,
    Topics::GetCandidate as u8,
    Topics::GetMempool as u8,
];

struct Response {
    msg: Message,
    recv_peer: SocketAddr,
}

#[derive(Default)]
pub struct DataBrokerSrv {
    /// A queue of pending requests to process.
    /// Request here is literally a GET message
    requests: AsyncQueue<Message>,
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
            tokio::select! {
                // Receives inbound wire messages.
                recv = &mut self.requests.recv() => {
                    if let Ok(msg) = recv {

                        let network = network.clone();
                        let db = db.clone();

                        tokio::spawn(async move {
                            match Self::handle_request(&network, &db, &msg).await {
                                Ok(resp) => {
                                    // Send response
                                    network.write().await.
                                        send_to_peer(&resp.msg, resp.recv_peer).await;
                                },
                                Err(e) => {
                                    tracing::warn!("error handling msg: {:?}", e);
                                }
                            }
                        });
                    }
                },
            }
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
                Ok(Response { msg, recv_peer })
            }
            _ => Err(anyhow::anyhow!("unhandled message")),
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

        match res {
            Some(block) => {
                Ok(Message::new_candidate_resp(payload::CandidateResp {
                    candidate: block,
                }))
            }
            None => Err(anyhow::anyhow!("could not find candidate block")),
        }
    }
}
