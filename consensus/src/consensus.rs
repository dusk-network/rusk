// This Source Code Form is subject to the terms of the Mozilla Public License,
// v. 2.0. If a copy of the MPL was not distributed with this file, You can
// obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::commons::{Block, ConsensusError, RoundUpdate};
use crate::phase::Phase;

use crate::agreement::step;
use crate::execution_ctx::ExecutionCtx;
use crate::messages::Message;
use crate::queue::Queue;
use crate::user::provisioners::Provisioners;
use crate::util::pending_queue::PendingQueue;
use crate::{config, selection};
use crate::{firststep, secondstep};
use tracing::{error, Instrument};

use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

pub struct Consensus {
    /// inbound is a queue of messages that comes from outside world
    inbound: PendingQueue,
    /// outbound_msgs is a queue of messages, this consensus instance shares
    /// with the outside world.
    outbound: PendingQueue,

    /// future_msgs is a queue of messages read from inbound_msgs queue. These
    /// msgs are pending to be handled in a future round/step.
    future_msgs: Arc<Mutex<Queue<Message>>>,

    /// agreement_layer implements Agreement message handler within the context
    /// of a separate task execution.
    agreement_process: step::Agreement,
}

impl Consensus {
    pub fn new(
        inbound: PendingQueue,
        outbound: PendingQueue,
        aggr_inbound_queue: PendingQueue,
        aggr_outbound_queue: PendingQueue,
    ) -> Self {
        Self {
            inbound,
            outbound,
            future_msgs: Arc::new(Mutex::new(Queue::default())),
            agreement_process: step::Agreement::new(aggr_inbound_queue, aggr_outbound_queue),
        }
    }

    fn spawn_main_loop(
        &mut self,
        ru: RoundUpdate,
        mut provisioners: Provisioners,
        mut agr_inbound_queue: PendingQueue,
    ) -> JoinHandle<Result<Block, ConsensusError>> {
        let inbound = self.inbound.clone();
        let outbound = self.outbound.clone();
        let future_msgs = self.future_msgs.clone();

        tokio::spawn(async move {
            if ru.round > 0 {
                future_msgs.lock().await.clear(ru.round - 1);
            }

            let mut phases = [
                Phase::Selection(selection::step::Selection::new()),
                Phase::Reduction1(firststep::step::Reduction::new()),
                Phase::Reduction2(secondstep::step::Reduction::new()),
            ];

            // Consensus loop
            // Initialize and run consensus loop
            let mut step: u8 = 0;

            loop {
                let mut msg = Message::empty();

                // Execute a single iteration
                for phase in phases.iter_mut() {
                    step += 1;

                    // Initialize new phase with message returned by previous phase.
                    phase.initialize(&msg, ru.round, step);

                    // Construct phase execution context
                    let ctx = ExecutionCtx::new(
                        inbound.clone(),
                        outbound.clone(),
                        future_msgs.clone(),
                        &mut provisioners,
                        ru,
                        step,
                    );

                    // Execute a phase.
                    // An error returned here terminates consensus
                    // round. This normally happens if consensus channel is cancelled by
                    // agreement loop on finding the winning block for this round.
                    msg = phase
                        .run(ctx)
                        .instrument(tracing::info_span!(
                            "main_task",
                            round = ru.round,
                            step = step,
                            pubkey = ru.pubkey_bls.encode_short_hex(),
                        ))
                        .await?;

                    if step >= config::CONSENSUS_MAX_STEP {
                        return Err(ConsensusError::MaxStepReached);
                    }
                }

                // Delegate (agreement) message result to agreement loop for
                // further processing.
                let _ = agr_inbound_queue
                    .send(msg.clone())
                    .await
                    .map_err(|e| error!("send agreement failed with {:?}", e));
            }
        })
    }

    /// Spin the consensus state machine. The consensus runs for the whole round
    /// until either a new round is produced or the node needs to re-sync. The
    /// Agreement loop (acting roundwise) runs concurrently with the
    /// generation-selection-reduction loop (acting step-wise).
    pub async fn spin(
        &mut self,
        ru: RoundUpdate,
        mut provisioners: Provisioners,
    ) -> Result<Block, ConsensusError> {
        // Enable/Disable all members stakes depending on the current round. If
        // a stake is not eligible for this round, it's disabled.
        provisioners.update_eligibility_flag(ru.round);

        // Agreement loop Executes agreement loop in a separate tokio::task to
        // collect (aggr)Agreement messages.
        let mut agreement_task_handle = self.agreement_process.spawn(ru, provisioners.clone());

        // Consensus loop - generation-selection-reduction loop
        let mut main_task_handle = self.spawn_main_loop(
            ru,
            provisioners,
            self.agreement_process.inbound_queue.clone(),
        );

        // Wait for any of the tasks to complete.
        let result;
        tokio::select! {
            recv = &mut agreement_task_handle => {
                result = recv.expect("wrong agreement_task handle");
                tracing::trace!("agreement result: {:?}", result);
            },
            recv = &mut main_task_handle => {
               result = recv.expect("wrong main_task handle");
                tracing::trace!("main_loop result: {:?}", result);
            }
        }

        // Cancel all tasks
        agreement_task_handle.abort();
        main_task_handle.abort();

        result
    }
}
