// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::commons::{
    AgreementSender, ConsensusError, Database, IterCounter, RoundUpdate,
};
use crate::contract_state::Operations;
use crate::phase::Phase;

use node_data::ledger::Block;

use node_data::message::{AsyncQueue, Message, Topics};

use crate::agreement::step;
use crate::execution_ctx::{ExecutionCtx, IterationCtx};
use crate::proposal;
use crate::queue::Queue;
use crate::user::provisioners::Provisioners;
use crate::{ratification, validation};
use tracing::Instrument;

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
        provisioners: Provisioners,
        cancel_rx: oneshot::Receiver<i32>,
    ) -> Result<Block, ConsensusError> {
        let round = ru.round;

        // Agreement loop Executes agreement loop in a separate tokio::task to
        // collect (aggr)Agreement messages.
        let mut agreement_task_handle = self.agreement_process.spawn(
            ru.clone(),
            provisioners.clone(),
            self.db.clone(),
        );

        let sender =
            AgreementSender::new(self.agreement_process.inbound_queue.clone());

        // Consensus loop - proposal-validation-ratificaton loop
        let mut main_task_handle =
            self.spawn_main_loop(ru, provisioners, sender);

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
        sender: AgreementSender,
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

            let proposal_handler =
                Arc::new(Mutex::new(proposal::handler::ProposalHandler::new(
                    db.clone(),
                    sv_registry.clone(),
                )));

            let validation_handler = Arc::new(Mutex::new(
                validation::handler::ValidationHandler::new(
                    db.clone(),
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
                    db.clone(),
                    validation_handler.clone(),
                )),
                Phase::Ratification(ratification::step::RatificationStep::new(
                    executor.clone(),
                    ratification_handler.clone(),
                )),
            ];

            // Consensus loop
            // Initialize and run consensus loop

            let mut iteration_counter: u8 = 0;
            let mut iter_ctx = IterationCtx::new(
                ru.round,
                iteration_counter,
                proposal_handler.clone(),
                validation_handler.clone(),
                ratification_handler.clone(),
            );

            loop {
                iter_ctx.on_begin(iteration_counter);

                let mut msg = Message::empty();
                // Execute a single iteration
                for pos in 0..phases.len() {
                    let phase = phases.get_mut(pos).unwrap();

                    let step = iteration_counter.step_from_pos(pos);
                    let name = phase.name();

                    // Initialize new phase with message returned by previous
                    // phase.
                    phase.reinitialize(&msg, ru.round, step).await;

                    // Construct phase execution context
                    let ctx = ExecutionCtx::new(
                        &mut iter_ctx,
                        inbound.clone(),
                        outbound.clone(),
                        future_msgs.clone(),
                        &mut provisioners,
                        ru.clone(),
                        step,
                        executor.clone(),
                        sv_registry.clone(),
                        sender.clone(),
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
                    // agreement is generated for a former or current iteration.
                    if msg.topic() == Topics::Quorum {
                        sender.send(msg.clone()).await;
                    }
                }

                iter_ctx.on_end();

                iteration_counter.next()?;
                // Delegate (agreement) message result to agreement loop for
                // further processing.
            }
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
