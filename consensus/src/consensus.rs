// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{ConsensusError, Database, RoundUpdate};
use crate::contract_state::Operations;
use crate::phase::Phase;
use node_data::ledger::{to_str, Block};

use node_data::message::{AsyncQueue, Message, Payload, Topics};

use crate::agreement::step;
use crate::execution_ctx::{ExecutionCtx, IterationCtx};
use crate::queue::Queue;
use crate::user::provisioners::Provisioners;
use crate::{config, selection};
use crate::{firststep, secondstep};
use tracing::{error, Instrument};

use crate::round_ctx::RoundCtx;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};
use tokio::task::JoinHandle;

pub struct Consensus<T: Operations, D: Database> {
    /// inbound is a queue of messages that comes from outside world
    inbound: AsyncQueue<Message>,
    /// outbound_msgs is a queue of messages, this consensus instance shares
    /// with the outside world.
    outbound: AsyncQueue<Message>,

    /// future_msgs is a queue of messages read from inbound_msgs queue. These
    /// msgs are pending to be handled in a future round/step.
    future_msgs: Arc<Mutex<Queue<Message>>>,

    /// agreement_layer implements Agreement message handler within the context
    /// of a separate task execution.
    agreement_process: step::Agreement,

    /// Reference to the executor of any EST-related call
    executor: Arc<Mutex<T>>,

    // Database
    db: Arc<Mutex<D>>,
}

impl<T: Operations + 'static, D: Database + 'static> Consensus<T, D> {
    /// Creates an instance of Consensus.
    ///
    /// # Arguments
    ///
    /// * `inbound` - a queue of input messages consumed by main loop
    /// * `outbound` - a queue of output messages that  main loop broadcasts to
    ///   the outside world
    ///
    /// * `agr_inbound_queue` - a queue of input messages consumed solely by
    ///   Agreement loop
    /// * `agr_outbound_queue` - a queue of output messages that Agreement loop
    ///   broadcasts to the outside world
    pub fn new(
        inbound: AsyncQueue<Message>,
        outbound: AsyncQueue<Message>,
        agr_inbound_queue: AsyncQueue<Message>,
        agr_outbound_queue: AsyncQueue<Message>,
        executor: Arc<Mutex<T>>,
        db: Arc<Mutex<D>>,
    ) -> Self {
        Self {
            inbound,
            outbound,
            future_msgs: Arc::new(Mutex::new(Queue::default())),
            agreement_process: step::Agreement::new(
                agr_inbound_queue,
                agr_outbound_queue,
            ),
            executor,
            db,
        }
    }

    /// Spins the consensus state machine. The consensus runs for the whole
    /// round until either a new round is produced or the node needs to re-sync.
    ///
    /// The Agreement loop (acting roundwise) runs concurrently with the
    /// generation-selection-reduction loop (acting step-wise).
    ///
    /// # Arguments
    ///
    /// * `provisioner` - a list of the provisioners based on the most recent
    ///   contract state.
    ///
    /// * `cancel_rx` - a chan that allows the client to drop consensus
    ///   execution on demand.
    pub async fn spin(
        &mut self,
        ru: RoundUpdate,
        mut provisioners: Provisioners,
        cancel_rx: oneshot::Receiver<i32>,
    ) -> Result<Block, ConsensusError> {
        let round = ru.round;
        // Enable/Disable all members stakes depending on the current round. If
        // a stake is not eligible for this round, it's disabled.
        provisioners.update_eligibility_flag(round);

        // Agreement loop Executes agreement loop in a separate tokio::task to
        // collect (aggr)Agreement messages.
        let mut agreement_task_handle = self.agreement_process.spawn(
            ru.clone(),
            provisioners.clone(),
            self.db.clone(),
        );

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
                result = recv.map_err(|_| ConsensusError::Canceled)?;
                tracing::trace!("agreement result: {:?}", result);
            },
            recv = &mut main_task_handle => {
                result = recv.map_err(|_| ConsensusError::Canceled)?;
                tracing::trace!("main_loop result: {:?}", result);
            },
            // Canceled from outside.
            // This could be triggered by Synchronizer or on node termination.
            _ = cancel_rx => {
                result = Err(ConsensusError::Canceled);
                tracing::debug!(event = "consensus canceled", round);
            }
        }

        // Tear-down procedure
        // Delete all candidates
        self.db.lock().await.delete_candidate_blocks();

        // Abort all tasks
        abort(&mut agreement_task_handle).await;
        abort(&mut main_task_handle).await;

        result
    }

    fn spawn_main_loop(
        &mut self,
        ru: RoundUpdate,
        mut provisioners: Provisioners,
        mut agr_inbound_queue: AsyncQueue<Message>,
    ) -> JoinHandle<Result<Block, ConsensusError>> {
        let inbound = self.inbound.clone();
        let outbound = self.outbound.clone();
        let future_msgs = self.future_msgs.clone();
        let executor = self.executor.clone();
        let db = self.db.clone();

        tokio::spawn(async move {
            if ru.round > 0 {
                future_msgs.lock().await.clear_round(ru.round - 1);
            }

            let round_ctx = Arc::new(Mutex::new(RoundCtx::new(ru.clone())));

            let mut phases = [
                Phase::Selection(selection::step::Selection::new(
                    executor.clone(),
                    db.clone(),
                )),
                Phase::Reduction1(firststep::step::Reduction::new(
                    executor.clone(),
                    db.clone(),
                    round_ctx.clone(),
                )),
                Phase::Reduction2(secondstep::step::Reduction::new(
                    executor,
                    round_ctx.clone(),
                )),
            ];

            // Consensus loop
            // Initialize and run consensus loop
            let mut step: u8 = 0;

            loop {
                let mut msg = Message::empty();
                let mut iter_ctx = IterationCtx::new(ru.round, step + 1);

                // Execute a single iteration
                for phase in phases.iter_mut() {
                    step += 1;
                    let name = phase.name();

                    // Initialize new phase with message returned by previous
                    // phase.
                    phase.reinitialize(&msg, ru.round, step);

                    // Construct phase execution context
                    let ctx = ExecutionCtx::new(
                        &mut iter_ctx,
                        inbound.clone(),
                        outbound.clone(),
                        future_msgs.clone(),
                        &mut provisioners,
                        ru.clone(),
                        step,
                    );

                    // Execute a phase.
                    // An error returned here terminates consensus
                    // round. This normally happens if consensus channel is
                    // cancelled by agreement loop on
                    // finding the winning block for this round.
                    msg = phase
                        .run(ctx)
                        .instrument(tracing::info_span!(
                            "main",
                            round = ru.round,
                            step = step,
                            name,
                            pk = ru.pubkey_bls.to_bs58(),
                        ))
                        .await?;

                    // During execution of any step we may encounter that an
                    // agreement is generated for previous iteration.
                    if msg.topic() == Topics::Agreement {
                        break;
                    }

                    if step >= config::CONSENSUS_MAX_STEP {
                        return Err(ConsensusError::MaxStepReached);
                    }
                }

                // Delegate (agreement) message result to agreement loop for
                // further processing.

                Self::send_agreement(&mut agr_inbound_queue, msg.clone()).await;
            }
        })
    }

    /// Sends an agreement (internally) to the agreement loop.
    async fn send_agreement(
        agr_inbound_queue: &mut AsyncQueue<Message>,
        msg: Message,
    ) {
        if let Payload::Agreement(payload) = &msg.payload {
            if payload.signature == [0u8; 48]
                || payload.first_step.is_empty()
                || payload.second_step.is_empty()
                || msg.header.block_hash == [0; 32]
            {
                return;
            }

            tracing::debug!(
                event = "send agreement",
                hash = to_str(&msg.header.block_hash),
                round = msg.header.round,
                step = msg.header.step,
                first = format!("{:#?}", payload.first_step),
                second = format!("{:#?}", payload.second_step),
                signature = to_str(&payload.signature),
            );

            let _ = agr_inbound_queue
                .send(msg.clone())
                .await
                .map_err(|e| error!("send agreement failed with {:?}", e));
        }
    }
}

#[inline]
async fn abort<T>(h: &mut JoinHandle<T>) {
    if h.is_finished() {
        return;
    }

    h.abort();

    let _ = h.await;
}
