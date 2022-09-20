// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::agreement::verifiers;
use crate::commons::{Hash, RoundUpdate};
use crate::messages;
use crate::messages::{payload, Message, Payload};
use crate::user::provisioners::Provisioners;
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tracing::error;

type Store = BTreeMap<u8, Vec<(payload::Agreement, usize)>>;
type AtomicStorePerHash = Arc<Mutex<BTreeMap<Hash, Store>>>;

pub(crate) struct Accumulator {
    stores: AtomicStorePerHash,
    workers: Vec<JoinHandle<()>>,
    inbound: Sender<Message>,
}

impl Accumulator {
    pub fn new(
        _workers_amount: usize,
        collected_votes_tx: Sender<Message>,
        provisioners: &mut Provisioners,
        ru: RoundUpdate,
    ) -> Self {
        let (tx, mut rx) = mpsc::channel::<Message>(100);

        let mut a = Self {
            stores: Default::default(),
            workers: vec![],
            inbound: tx,
        };

        let stores = a.stores.clone();
        let mut provisioners = provisioners.clone();

        // Spawn a single worker to process all agreement message verifications
        // It does also accumulate results and exists by providing a final CollectVotes message back to Agreement loop.
        let handle = tokio::spawn(async move {
            // Process each request for verification
            while let Some(msg) = rx.recv().await {
                if let Err(e) =
                    verifiers::verify_agreement(msg.clone(), &mut provisioners, ru.seed).await
                {
                    error!("{:#?}", e);
                    continue;
                }

                if let Some(msg) = Self::accumulate(stores.clone(), msg).await {
                    collected_votes_tx.send(msg).await.unwrap_or_else(|err| {
                        error!("unable to send_msg collected_votes {:?}", err)
                    });
                    break;
                }
            }
        });

        a.workers.push(handle);
        a
    }

    pub async fn process(&mut self, msg: Message) {
        // To follow strictly the initial design we need to delegate the task to a workers_pool.
        self.inbound
            .send(msg)
            .await
            .unwrap_or_else(|err| error!("unable to queue agreement_msg {:?}", err));
    }

    async fn accumulate(
        stores: AtomicStorePerHash,
        msg: messages::Message,
    ) -> Option<messages::Message> {
        let hdr = msg.header;

        match msg.payload {
            Payload::Agreement(payload) => {
                //TODO: let weight := a.handler.VotesFor(hdr.PubKeyBLS, hdr.Round, hdr.Step)
                let weight = 0;
                stores
                    .lock()
                    .await
                    .entry(hdr.block_hash)
                    .or_insert(Store::default())
                    .entry(hdr.step)
                    .or_insert(vec![(payload, weight)]);

                // TODO: 	if s.contains(idx, a) {

                // TODO: if count >= a.handler.Quorum(hdr.Round) {}
            }
            _ => {}
        };

        None
    }
}

impl Drop for Accumulator {
    fn drop(&mut self) {
        // Abort all workers
        for handle in self.workers.iter() {
            handle.abort();
        }

        self.workers.clear();
    }
}
