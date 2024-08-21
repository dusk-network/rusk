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
use dusk_consensus::operations::Voter;
use dusk_consensus::quorum::verifiers;
use dusk_consensus::quorum::verifiers::QuorumResult;
use dusk_consensus::user::committee::CommitteeSet;
use dusk_consensus::user::provisioners::{ContextProvisioners, Provisioners};
use execution_core::stake::EPOCH;
use node_data::ledger::{Fault, InvalidFault, Seed, Signature};
use node_data::message::payload::{RatificationResult, Vote};
use node_data::message::ConsensusHeader;
use node_data::{ledger, StepName};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::info;

const MARGIN_TIMESTAMP: u64 = 3;

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
    /// Returns the number of Previous Non-Attested Iterations (PNI)
    pub async fn execute_checks(
        &self,
        candidate_block: &ledger::Header,
        disable_att_check: bool,
    ) -> anyhow::Result<(u8, Vec<Voter>, Vec<Voter>)> {
        self.verify_basic_fields(candidate_block).await?;

        let prev_block_voters =
            self.verify_prev_block_cert(candidate_block).await?;

        let mut candidate_block_voters = vec![];
        if !disable_att_check {
            (_, _, candidate_block_voters) = verify_att(
                &candidate_block.att,
                candidate_block.to_consensus_header(),
                self.prev_header.seed,
                self.provisioners.current(),
                RatificationResult::Success(Vote::Valid(candidate_block.hash)),
            )
            .await?;
        }

        let pni = self.verify_failed_iterations(candidate_block).await?;
        Ok((pni, prev_block_voters, candidate_block_voters))
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
        let pk =
            execution_core::signatures::bls::PublicKey::from_bytes(pk_bytes)
                .map_err(|err| anyhow!("invalid pk bytes: {:?}", err))?;

        let signature =
            execution_core::signatures::bls::MultisigSignature::from_bytes(
                seed,
            )
            .map_err(|err| anyhow!("invalid signature bytes: {}", err))?;

        execution_core::signatures::bls::MultisigPublicKey::aggregate(&[pk])
            .map_err(|err| anyhow!("failed aggregating single key: {}", err))?
            .verify(&signature, &self.prev_header.seed.inner()[..])
            .map_err(|err| anyhow!("invalid seed: {:?}", err))?;

        Ok(())
    }

    pub async fn verify_prev_block_cert(
        &self,
        candidate_block: &'a ledger::Header,
    ) -> anyhow::Result<Vec<Voter>> {
        if self.prev_header.height == 0 {
            return Ok(vec![]);
        }

        let prev_block_hash = candidate_block.prev_block_hash;

        let prev_block_seed = self.db.read().await.view(|v| {
            v.fetch_block_header(&self.prev_header.prev_block_hash)?
                .ok_or_else(|| anyhow::anyhow!("Header not found"))
                .map(|h| h.seed)
        })?;

        let (_, _, voters) = verify_att(
            &candidate_block.prev_block_cert,
            self.prev_header.to_consensus_header(),
            prev_block_seed,
            self.provisioners.prev(),
            RatificationResult::Success(Vote::Valid(prev_block_hash)),
        )
        .await?;

        Ok(voters)
    }

    /// Return the number of failed iterations that have no quorum in the
    /// ratification phase
    ///
    /// We refer to this number as Previous Non-Attested Iterations, or PNI
    pub async fn verify_failed_iterations(
        &self,
        candidate_block: &'a ledger::Header,
    ) -> anyhow::Result<u8> {
        let mut failed_atts = 0u8;

        for (iter, att) in candidate_block
            .failed_iterations
            .att_list
            .iter()
            .enumerate()
        {
            if let Some((att, pk)) = att {
                info!(event = "verify_att", att_type = "failed_att", iter);

                let expected_pk = self.provisioners.current().get_generator(
                    iter as u8,
                    self.prev_header.seed,
                    candidate_block.height,
                );

                anyhow::ensure!(pk == &expected_pk, "Invalid generator. Expected {expected_pk:?}, actual {pk:?}");

                let mut consensus_header =
                    candidate_block.to_consensus_header();
                consensus_header.iteration = iter as u8;

                let (_, rat_quorum, _) = verify_att(
                    att,
                    consensus_header,
                    self.prev_header.seed,
                    self.provisioners.current(),
                    RatificationResult::Fail(Vote::default()),
                )
                .await?;

                if rat_quorum.quorum_reached() {
                    failed_atts += 1;
                }
            }
        }

        Ok(candidate_block.iteration - failed_atts)
    }

    /// Extracts voters list of a block.
    ///
    /// Returns a list of voters with their credits for both ratification and
    /// validation step
    pub async fn get_voters(
        blk: &'a ledger::Header,
        provisioners: &Provisioners,
        prev_block_seed: Seed,
    ) -> Vec<Voter> {
        let att = &blk.att;
        let consensus_header = blk.to_consensus_header();

        let committee = RwLock::new(CommitteeSet::new(provisioners));

        let validation_voters = verifiers::get_step_voters(
            &consensus_header,
            &att.validation,
            &committee,
            prev_block_seed,
            StepName::Validation,
        )
        .await;

        let ratification_voters = verifiers::get_step_voters(
            &consensus_header,
            &att.ratification,
            &committee,
            prev_block_seed,
            StepName::Ratification,
        )
        .await;

        merge_voters(validation_voters, ratification_voters)
    }

    /// Verify faults inside a block.
    pub async fn verify_faults(
        &self,
        current_height: u64,
        faults: &[Fault],
    ) -> Result<(), InvalidFault> {
        verify_faults(self.db.clone(), current_height, faults).await
    }
}

pub async fn verify_faults<DB: database::DB>(
    db: Arc<RwLock<DB>>,
    current_height: u64,
    faults: &[Fault],
) -> Result<(), InvalidFault> {
    for f in faults {
        let fault_header = f.validate(current_height)?;
        db.read()
            .await
            .view(|db| {
                let prev_header = db
                    .fetch_block_header(&fault_header.prev_block_hash)?
                    .ok_or(anyhow::anyhow!("Slashing a non accepted header"))?;
                // No overflow here, since the header has been already validated
                // not to be 0
                if prev_header.height != fault_header.round - 1 {
                    anyhow::bail!("Invalid height for fault");
                }

                // FIX_ME: Instead of fetching all store faults, check the fault
                // id directly This needs the fault id to be
                // changed into "HEIGHT|TYPE|PROV_KEY"
                let start_height = fault_header.round.saturating_sub(EPOCH);
                let stored_faults = db.fetch_faults_by_block(start_height)?;
                if stored_faults.iter().any(|other| f.same(other)) {
                    anyhow::bail!("Double fault detected");
                }

                Ok(())
            })
            .map_err(|e| InvalidFault::Other(format!("{e:?}")))?;
    }
    Ok(())
}

pub async fn verify_att(
    att: &ledger::Attestation,
    consensus_header: ConsensusHeader,
    curr_seed: Signature,
    curr_eligible_provisioners: &Provisioners,
    expected_result: RatificationResult,
) -> anyhow::Result<(QuorumResult, QuorumResult, Vec<Voter>)> {
    // Check expected result
    match (att.result, expected_result) {
        // Both are Success and the inner Valid(Hash) values match
        (
            RatificationResult::Success(Vote::Valid(r_hash)),
            RatificationResult::Success(Vote::Valid(e_hash)),
        ) => {
            if r_hash != e_hash {
                anyhow::bail!(
                    "Invalid Attestation: Expected block hash: {:?}, Got: {:?}",
                    e_hash,
                    r_hash
                )
            }
        }
        // Both are Fail
        (RatificationResult::Fail(_), RatificationResult::Fail(_)) => {}
        // All other mismatches
        _ => anyhow::bail!(
            "Invalid Attestation: Result: {:?}, Expected: {:?}",
            att.result,
            expected_result
        ),
    }
    let committee = RwLock::new(CommitteeSet::new(curr_eligible_provisioners));

    let vote = att.result.vote();

    // Verify validation
    let (val_result, validation_voters) = verifiers::verify_step_votes(
        &consensus_header,
        vote,
        &att.validation,
        &committee,
        curr_seed,
        StepName::Validation,
    )
    .await?;

    // Verify ratification
    let (rat_result, ratification_voters) = verifiers::verify_step_votes(
        &consensus_header,
        vote,
        &att.ratification,
        &committee,
        curr_seed,
        StepName::Ratification,
    )
    .await?;

    let voters = merge_voters(validation_voters, ratification_voters);
    Ok((val_result, rat_result, voters))
}

/// Merges two Vec<Voter>, summing up the usize values if the PublicKey is
/// repeated
fn merge_voters(v1: Vec<Voter>, v2: Vec<Voter>) -> Vec<Voter> {
    let mut voter_map = BTreeMap::new();

    for (pk, count) in v1.into_iter().chain(v2.into_iter()) {
        let counter = voter_map.entry(pk).or_default();
        *counter += count;
    }

    voter_map.into_iter().collect()
}
