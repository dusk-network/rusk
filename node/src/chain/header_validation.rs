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
use dusk_consensus::operations::VoterWithCredits;
use dusk_consensus::quorum::verifiers;
use dusk_consensus::quorum::verifiers::QuorumResult;
use dusk_consensus::user::committee::{Committee, CommitteeSet};
use dusk_consensus::user::provisioners::{ContextProvisioners, Provisioners};
use execution_core::stake::EPOCH;
use node_data::ledger::{to_str, Fault, InvalidFault, Seed, Signature};
use node_data::message::payload::{RatificationResult, Vote};
use node_data::message::ConsensusHeader;
use node_data::{ledger, StepName};
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
        disable_winner_att_check: bool,
    ) -> anyhow::Result<(u8, Vec<VoterWithCredits>, Vec<VoterWithCredits>)>
    {
        self.verify_basic_fields(candidate_block).await?;
        let prev_block_voters =
            self.verify_prev_block_cert(candidate_block).await?;

        let mut candidate_block_voters = vec![];
        if !disable_winner_att_check {
            candidate_block_voters =
                self.verify_success_att(candidate_block).await?;
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
        let pk = execution_core::BlsPublicKey::from_bytes(pk_bytes)
            .map_err(|err| anyhow!("invalid pk bytes: {:?}", err))?;

        let signature = execution_core::BlsSignature::from_bytes(seed)
            .map_err(|err| anyhow!("invalid signature bytes: {}", err))?;

        execution_core::BlsAggPublicKey::from(&pk)
            .verify(&signature, &self.prev_header.seed.inner()[..])
            .map_err(|err| anyhow!("invalid seed: {:?}", err))?;

        Ok(())
    }

    pub async fn verify_prev_block_cert(
        &self,
        candidate_block: &'a ledger::Header,
    ) -> anyhow::Result<Vec<VoterWithCredits>> {
        if self.prev_header.height == 0 {
            return Ok(vec![]);
        }

        let prev_block_seed = self.db.read().await.view(|v| {
            v.fetch_block_header(&self.prev_header.prev_block_hash)?
                .ok_or_else(|| anyhow::anyhow!("Header not found"))
                .map(|h| h.seed)
        })?;

        let cert_result = candidate_block.prev_block_cert.result;
        let prev_block_hash = candidate_block.prev_block_hash;

        match candidate_block.prev_block_cert.result {
            RatificationResult::Success(Vote::Valid(hash))
                if hash == prev_block_hash => {}
            _ => anyhow::bail!(
                "Invalid result for previous block hash: {cert_result:?}"
            ),
        }

        let (_, _, voters) = verify_att(
            &candidate_block.prev_block_cert,
            self.prev_header.prev_block_hash,
            self.prev_header.height,
            self.prev_header.iteration,
            prev_block_seed,
            self.provisioners.prev(),
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

                if let RatificationResult::Success(_) = att.result {
                    anyhow::bail!("Failed iterations should not contains a RatificationResult::Success");
                }

                let expected_pk = self.provisioners.current().get_generator(
                    iter as u8,
                    self.prev_header.seed,
                    candidate_block.height,
                );

                anyhow::ensure!(pk == &expected_pk, "Invalid generator. Expected {expected_pk:?}, actual {pk:?}");

                let (_, rat_quorum, _) = verify_att(
                    att,
                    self.prev_header.hash,
                    candidate_block.height,
                    iter as u8,
                    self.prev_header.seed,
                    self.provisioners.current(),
                )
                .await?;

                if rat_quorum.quorum_reached() {
                    failed_atts += 1;
                }
            }
        }

        Ok(candidate_block.iteration - failed_atts)
    }

    pub async fn verify_success_att(
        &self,
        candidate_block: &'a ledger::Header,
    ) -> anyhow::Result<Vec<VoterWithCredits>> {
        let (_, _, voters) = verify_att(
            &candidate_block.att,
            self.prev_header.hash,
            candidate_block.height,
            candidate_block.iteration,
            self.prev_header.seed,
            self.provisioners.current(),
        )
        .await?;

        Ok(voters)
    }

    /// Extracts voters list of a block.
    ///
    /// Returns a list of voters with their credits for both ratification and
    /// validation step
    pub async fn get_voters(
        blk: &'a ledger::Header,
        provisioners: &Provisioners,
        prev_block_seed: Seed,
    ) -> anyhow::Result<Vec<VoterWithCredits>> {
        let (_, _, voters) = verify_att(
            &blk.att,
            blk.prev_block_hash,
            blk.height,
            blk.iteration,
            prev_block_seed,
            provisioners,
        )
        .await?;

        Ok(voters)
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
                if prev_header.height != fault_header.round - 1 {
                    anyhow::bail!("Invalid height for fault");
                }

                // FIX_ME: Instead of fetching all store faults, check the fault
                // id directly This needs the fault id to be
                // changed into "HEIGHT|TYPE|PROV_KEY"
                let stored_faults =
                    db.fetch_faults_by_block(fault_header.round - EPOCH)?;
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
    prev_block_hash: [u8; 32],
    round: u64,
    iteration: u8,
    curr_seed: Signature,
    curr_eligible_provisioners: &Provisioners,
) -> anyhow::Result<(QuorumResult, QuorumResult, Vec<VoterWithCredits>)> {
    let committee = RwLock::new(CommitteeSet::new(curr_eligible_provisioners));

    let mut result = (QuorumResult::default(), QuorumResult::default());

    let consensus_header = ConsensusHeader {
        iteration,
        round,
        prev_block_hash,
    };
    let v_committee;
    let r_committee;

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
        Ok((validation_quorum_result, committee)) => {
            result.0 = validation_quorum_result;
            v_committee = committee;
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
        Ok((ratification_quorum_result, committee)) => {
            result.1 = ratification_quorum_result;
            r_committee = committee;
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

    let voters = merge_committees(&v_committee, &r_committee);
    Ok((result.0, result.1, voters))
}

/// Merges two committees into a vector
fn merge_committees(a: &Committee, b: &Committee) -> Vec<VoterWithCredits> {
    let mut members = a.members().clone();
    for (key, value) in b.members() {
        // Keeps track of the number of occurrences for each member.
        let counter = members.entry(key.clone()).or_insert(0);
        *counter += *value;
    }

    members
        .into_iter()
        .map(|(key, credits)| (*key.inner(), credits))
        .collect()
}
