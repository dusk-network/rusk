// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{ConsensusError, Database, QuorumMsgSender, RoundUpdate};
use crate::config::CONSENSUS_MAX_ITER;
use crate::operations::Operations;
use crate::phase::Phase;

use node_data::ledger::Block;

use node_data::message::{AsyncQueue, Message, Topics};

use crate::execution_ctx::ExecutionCtx;
use crate::proposal;
use crate::queue::Queue;
use crate::quorum::task;
use crate::user::provisioners::Provisioners;
use crate::{ratification, validation};
use tracing::Instrument;

use crate::iteration_ctx::IterationCtx;
use crate::step_votes_reg::CertInfoRegistry;
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

    /// quorum_process implements Quorum message handler within the context
    /// of a separate task execution.
    quorum_process: task::Quorum,

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
    /// * `quorum_inbound_queue` - a queue of input messages consumed solely by
    ///   Quorum loop
    /// * `quorum_outbound_queue` - a queue of output messages that Quorum loop
    ///   broadcasts to the outside world
    pub fn new(
        inbound: AsyncQueue<Message>,
        outbound: AsyncQueue<Message>,
        quorum_inbound_queue: AsyncQueue<Message>,
        quorum_outbound_queue: AsyncQueue<Message>,
        executor: Arc<Mutex<T>>,
        db: Arc<Mutex<D>>,
    ) -> Self {
        Self {
            inbound,
            outbound,
            future_msgs: Arc::new(Mutex::new(Queue::default())),
            quorum_process: task::Quorum::new(
                quorum_inbound_queue,
                quorum_outbound_queue,
            ),
            executor,
            db,
        }
    }

    /// Spins the consensus state machine. The consensus runs for the whole
    /// round until either a new round is produced or the node needs to re-sync.
    ///
    /// # Arguments
    ///
    /// * `provisioner` - a list of the provisioners based on the most recent
    ///   contract state.
    ///
    /// * `cancel_rx` - a chan that allows the client to drop consensus
    ///   execution on demand.
    pub async fn spin(
        &self,
        ru: RoundUpdate,
        provisioners: Arc<Provisioners>,
        cancel_rx: oneshot::Receiver<i32>,
    ) -> Result<Block, ConsensusError> {
        let round = ru.round;

        let mut quorum_task_handle = self.quorum_process.spawn(
            ru.clone(),
            provisioners.clone(),
            self.db.clone(),
        );

        let sender =
            QuorumMsgSender::new(self.quorum_process.inbound_queue.clone());

        // Consensus loop - proposal-validation-ratificaton loop
        let mut main_task_handle =
            self.spawn_main_loop(ru, provisioners, sender);

        // Wait for any of the tasks to complete.
        let result;
        tokio::select! {
            recv = &mut quorum_task_handle => {
                result = recv.map_err(|_| ConsensusError::Canceled)?;
                tracing::trace!("quorum result: {:?}", result);
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
        abort(&mut quorum_task_handle).await;
        abort(&mut main_task_handle).await;

        result
    }

    fn spawn_main_loop(
        &self,
        ru: RoundUpdate,
        provisioners: Arc<Provisioners>,
        sender: QuorumMsgSender,
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

            let sv_registry =
                Arc::new(Mutex::new(CertInfoRegistry::new(ru.clone())));

            let proposal_handler = Arc::new(Mutex::new(
                proposal::handler::ProposalHandler::new(db.clone()),
            ));

            let validation_handler = Arc::new(Mutex::new(
                validation::handler::ValidationHandler::new(
                    sv_registry.clone(),
                ),
            ));

            let ratification_handler = Arc::new(Mutex::new(
                ratification::handler::RatificationHandler::new(
                    sv_registry.clone(),
                ),
            ));

            let mut phases = [
                Phase::Proposal(proposal::step::ProposalStep::new(
                    executor.clone(),
                    db.clone(),
                    proposal_handler.clone(),
                )),
                Phase::Validation(validation::step::ValidationStep::new(
                    executor.clone(),
                    validation_handler.clone(),
                )),
                Phase::Ratification(ratification::step::RatificationStep::new(
                    executor.clone(),
                    ratification_handler.clone(),
                )),
            ];

            // Consensus loop
            // Initialize and run consensus loop

            let mut iter: u8 = 0;
            let mut iter_ctx = IterationCtx::new(
                ru.round,
                iter,
                proposal_handler,
                validation_handler,
                ratification_handler,
                ru.base_timeouts.clone(),
            );

            while iter < CONSENSUS_MAX_ITER {
                iter_ctx.on_begin(iter);

                let mut msg = Message::empty();
                // Execute a single iteration
                for phase in phases.iter_mut() {
                    let step_name = phase.to_step_name();
                    // Initialize new phase with message returned by previous
                    // phase.
                    phase.reinitialize(msg, ru.round, iter).await;

                    // Construct phase execution context
                    let ctx = ExecutionCtx::new(
                        &mut iter_ctx,
                        inbound.clone(),
                        outbound.clone(),
                        future_msgs.clone(),
                        provisioners.as_ref(),
                        ru.clone(),
                        iter,
                        step_name,
                        executor.clone(),
                        sv_registry.clone(),
                        sender.clone(),
                    );

                    // Execute a phase.
                    // An error returned here terminates consensus
                    // round. This normally happens if consensus channel is
                    // cancelled by quorum loop on
                    // finding the winning block for this round.
                    msg = phase
                        .run(ctx)
                        .instrument(tracing::info_span!(
                            "main",
                            round = ru.round,
                            iter = iter,
                            name = ?step_name,
                            pk = ru.pubkey_bls.to_bs58(),
                        ))
                        .await?;

                    // During execution of any step we may encounter that an
                    // quorum is generated for a former or current iteration.
                    if msg.topic() == Topics::Quorum {
                        sender.send(msg.clone()).await;
                    }
                }

                iter_ctx.on_close();

                iter += 1;
                // Delegate (quorum) message result to quorum loop for
                // further processing.
            }
            Err(ConsensusError::MaxIterationReached)
        })
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
