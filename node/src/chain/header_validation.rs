// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::database;
use crate::database::Ledger;
use anyhow::anyhow;
use dusk_bytes::Serializable;
use dusk_consensus::config::MINIMUM_BLOCK_TIME;
use dusk_consensus::quorum::verifiers;
use dusk_consensus::quorum::verifiers::QuorumResult;
use dusk_consensus::user::committee::CommitteeSet;
use dusk_consensus::user::provisioners::{ContextProvisioners, Provisioners};
use node_data::ledger::to_str;
use node_data::ledger::Signature;
use node_data::message::payload::RatificationResult;
use node_data::message::ConsensusHeader;
use node_data::{ledger, StepName};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::info;

const MARGIN_TIMESTAMP: u64 = 10;

// TODO: Use thiserror instead of anyhow

#[derive(Debug, Error)]
enum HeaderVerificationErr {}

/// An implementation of the all validation checks of a candidate block header
/// according to current context
pub(crate) struct Validator<'a, DB: database::DB> {
    pub(crate) db: Arc<RwLock<DB>>,
    prev_header: &'a ledger::Header,
    provisioners: &'a ContextProvisioners,
}

impl<'a, DB: database::DB> Validator<'a, DB> {
    pub fn new(
        db: Arc<RwLock<DB>>,
        prev_header: &'a ledger::Header,
        provisioners: &'a ContextProvisioners,
    ) -> Self {
        Self {
            db,
            prev_header,
            provisioners,
        }
    }

    /// Executes check points to make sure a candidate header is fully valid
    ///
    /// * `disable_winner_att_check` - disables the check of the winning
    /// attestation
    ///
    /// Returns true if there is a attestation for each failed iteration, and if
    /// that attestation has a quorum in the ratification phase.
    ///
    /// If there are no failed iterations, it returns true
    pub async fn execute_checks(
        &self,
        candidate_block: &'_ ledger::Header,
        disable_winner_att_check: bool,
    ) -> anyhow::Result<bool> {
        self.verify_basic_fields(candidate_block).await?;
        self.verify_prev_block_cert(candidate_block).await?;

        if !disable_winner_att_check {
            self.verify_winning_att(candidate_block).await?;
        }

        self.verify_failed_iterations(candidate_block).await
    }

    /// Verifies any non-attestation field
    pub async fn verify_basic_fields(
        &self,
        candidate_block: &'a ledger::Header,
    ) -> anyhow::Result<()> {
        if candidate_block.version > 0 {
            return Err(anyhow!("unsupported block version"));
        }

        if candidate_block.hash == [0u8; 32] {
            return Err(anyhow!("empty block hash"));
        }

        if candidate_block.height != self.prev_header.height + 1 {
            return Err(anyhow!(
                "invalid block height block_height: {:?}, curr_height: {:?}",
                candidate_block.height,
                self.prev_header.height,
            ));
        }

        // Ensure rule of minimum block time is addressed
        if candidate_block.timestamp
            < self.prev_header.timestamp + MINIMUM_BLOCK_TIME
        {
            return Err(anyhow!("block time is less than minimum block time"));
        }

        let local_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|n| n.as_secs())
            .expect("valid unix epoch");

        if candidate_block.timestamp > local_time + MARGIN_TIMESTAMP {
            return Err(anyhow!(
                "block timestamp {} is higher than local time",
                candidate_block.timestamp
            ));
        }

        if candidate_block.prev_block_hash != self.prev_header.hash {
            return Err(anyhow!("invalid previous block hash"));
        }

        // Ensure block is not already in the ledger
        self.db.read().await.view(|v| {
            if Ledger::get_block_exists(&v, &candidate_block.hash)? {
                return Err(anyhow!("block already exists"));
            }

            Ok(())
        })?;

        // Verify seed field
        self.verify_seed_field(
            candidate_block.seed.inner(),
            candidate_block.generator_bls_pubkey.inner(),
        )?;

        Ok(())
    }

    fn verify_seed_field(
        &self,
        seed: &[u8; 48],
        pk_bytes: &[u8; 96],
    ) -> anyhow::Result<()> {
        let pk = execution_core::StakePublicKey::from_bytes(pk_bytes)
            .map_err(|err| anyhow!("invalid pk bytes: {:?}", err))?;

        let signature = execution_core::StakeSignature::from_bytes(seed)
            .map_err(|err| anyhow!("invalid signature bytes: {}", err))?;

        execution_core::StakeAggPublicKey::from(&pk)
            .verify(&signature, &self.prev_header.seed.inner()[..])
            .map_err(|err| anyhow!("invalid seed: {:?}", err))?;

        Ok(())
    }

    pub async fn verify_prev_block_cert(
        &self,
        candidate_block: &'a ledger::Header,
    ) -> anyhow::Result<()> {
        if self.prev_header.height == 0 {
            return Ok(());
        }

        let prev_block_seed = self.db.read().await.view(|v| {
            let prior_tip =
                Ledger::fetch_block_by_height(&v, self.prev_header.height - 1)?
                    .ok_or_else(|| anyhow::anyhow!("could not fetch block"))?;

            Ok::<_, anyhow::Error>(prior_tip.header().seed)
        })?;

        verify_block_att(
            self.prev_header.prev_block_hash,
            prev_block_seed,
            self.provisioners.prev(),
            self.prev_header.height,
            &candidate_block.prev_block_cert,
            self.prev_header.iteration,
        )
        .await?;

        Ok(())
    }

    /// Return true if there is a attestation for each failed iteration, and if
    /// that attestation has a quorum in the ratification phase.
    ///
    /// If there are no failed iterations, it returns true
    pub async fn verify_failed_iterations(
        &self,
        candidate_block: &'a ledger::Header,
    ) -> anyhow::Result<bool> {
        // Verify Failed iterations
        let mut all_failed = true;

        for (iter, att) in candidate_block
            .failed_iterations
            .att_list
            .iter()
            .enumerate()
        {
            if let Some((att, pk)) = att {
                info!(event = "verify_att", att_type = "failed_att", iter);

                if let RatificationResult::Success(_) = att.result {
                    anyhow::bail!("Failed iterations should not contains a RatificationResult::Success");
                }

                let expected_pk = self.provisioners.current().get_generator(
                    iter as u8,
                    self.prev_header.seed,
                    candidate_block.height,
                );

                anyhow::ensure!(pk == &expected_pk, "Invalid generator. Expected {expected_pk:?}, actual {pk:?}");

                let quorums = verify_block_att(
                    self.prev_header.hash,
                    self.prev_header.seed,
                    self.provisioners.current(),
                    candidate_block.height,
                    att,
                    iter as u8,
                )
                .await?;

                // Ratification quorum is enough to consider the iteration
                // failed
                all_failed = all_failed && quorums.1.quorum_reached();
            } else {
                all_failed = false;
            }
        }

        Ok(all_failed)
    }

    pub async fn verify_winning_att(
        &self,
        candidate_block: &'a ledger::Header,
    ) -> anyhow::Result<()> {
        verify_block_att(
            self.prev_header.hash,
            self.prev_header.seed,
            self.provisioners.current(),
            candidate_block.height,
            &candidate_block.att,
            candidate_block.iteration,
        )
        .await?;

        Ok(())
    }
}

pub async fn verify_block_att(
    prev_block_hash: [u8; 32],
    curr_seed: Signature,
    curr_eligible_provisioners: &Provisioners,
    round: u64,
    att: &ledger::Attestation,
    iteration: u8,
) -> anyhow::Result<(QuorumResult, QuorumResult)> {
    let committee = RwLock::new(CommitteeSet::new(curr_eligible_provisioners));

    let mut result = (QuorumResult::default(), QuorumResult::default());

    let consensus_header = ConsensusHeader {
        iteration,
        round,
        prev_block_hash,
    };
    let vote = att.result.vote();
    // Verify validation
    match verifiers::verify_step_votes(
        &consensus_header,
        vote,
        &att.validation,
        &committee,
        curr_seed,
        StepName::Validation,
    )
    .await
    {
        Ok(validation_quorum_result) => {
            result.0 = validation_quorum_result;
        }
        Err(e) => {
            return Err(anyhow!(
                "invalid validation, vote = {:?}, round = {}, iter = {}, seed = {},  sv = {:?}, err = {}",
                vote,
                round,
                iteration,
                to_str(curr_seed.inner()),
                att.validation,
                e
            ));
        }
    };

    // Verify ratification
    match verifiers::verify_step_votes(
        &consensus_header,
        vote,
        &att.ratification,
        &committee,
        curr_seed,
        StepName::Ratification,
    )
    .await
    {
        Ok(ratification_quorum_result) => {
            result.1 = ratification_quorum_result;
        }
        Err(e) => {
            return Err(anyhow!(
                "invalid ratification, vote = {:?}, round = {}, iter = {}, seed = {},  sv = {:?}, err = {}",
                vote,
                round,
                iteration,
                to_str(curr_seed.inner()),
                att.ratification,
                e,
            ));
        }
    }

    Ok(result)
}
