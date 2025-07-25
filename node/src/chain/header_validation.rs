// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::cmp;
use std::collections::BTreeMap;
use std::sync::Arc;

use dusk_bytes::Serializable;
use dusk_consensus::config::{
    is_emergency_block, is_emergency_iter, CONSENSUS_MAX_ITER,
    MINIMUM_BLOCK_TIME, MIN_EMERGENCY_BLOCK_TIME, RELAX_ITERATION_THRESHOLD,
};
use dusk_consensus::errors::{
    AttestationError, FailedIterationError, HeaderError,
};
use dusk_consensus::operations::Voter;
use dusk_consensus::quorum::verifiers;
use dusk_consensus::user::committee::CommitteeSet;
use dusk_consensus::user::provisioners::{ContextProvisioners, Provisioners};
use dusk_core::signatures::bls::{
    MultisigPublicKey, MultisigSignature, PublicKey as BlsPublicKey,
};
use dusk_core::stake::EPOCH;
use hex;
use node_data::bls::PublicKeyBytes;
use node_data::ledger::{Fault, InvalidFault, Seed};
use node_data::message::payload::{RatificationResult, Vote};
use node_data::message::{ConsensusHeader, BLOCK_HEADER_VERSION};
use node_data::{get_current_timestamp, ledger, StepName};
use tokio::sync::RwLock;
use tracing::{debug, error};

use crate::database;
use crate::database::Ledger;

const MARGIN_TIMESTAMP: u64 = 3;

// TODO: Use thiserror instead of anyhow

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

    /// Verifies all header fields except state root and event bloom
    ///
    /// # Arguments
    ///
    /// * `header` - the block header to verify
    /// * `expected_generator` - the generator extracted by the Deterministic
    ///   Sortition algorithm
    /// * `check_attestation` - `true` if the Attestation has to be verified
    ///   (this should be `false` for Candidate headers, for which no
    ///   Attestation has been yet produced)
    ///
    /// # Returns
    ///
    /// * the number of Previous Non-Attested Iterations (PNI)
    /// * the voters of the previous block Certificate
    /// * the voters of current block Attestation (if `check_attestation` is
    ///   `true`)
    pub async fn verify_block_header_fields(
        &self,
        header: &ledger::Header,
        expected_generator: &PublicKeyBytes,
        check_attestation: bool,
    ) -> Result<(u8, Vec<Voter>, Vec<Voter>), HeaderError> {
        self.verify_block_header_basic_fields(header, expected_generator)
            .await?;

        let cert_voters = self.verify_prev_block_cert(header).await?;

        let mut att_voters = vec![];
        if check_attestation {
            att_voters = verify_att(
                &header.att,
                header.to_consensus_header(),
                self.prev_header.seed,
                self.provisioners.current(),
                Some(RatificationResult::Success(Vote::Valid(header.hash))),
            )
            .await?;
        }

        let pni = self.verify_failed_iterations(header).await?;
        Ok((pni, cert_voters, att_voters))
    }

    fn verify_block_generator(
        &self,
        header: &'a ledger::Header,
        expected_generator: &PublicKeyBytes,
    ) -> Result<MultisigPublicKey, HeaderError> {
        if expected_generator != &header.generator_bls_pubkey {
            return Err(HeaderError::InvalidBlockSignature(
                "Signed by a different generator:".into(),
            ));
        }

        // Get generator MultisigPublicKey
        let generator = header.generator_bls_pubkey.inner();
        let generator = BlsPublicKey::from_bytes(generator).map_err(|err| {
            HeaderError::InvalidBlockSignature(format!(
                "invalid pk bytes: {err:?}"
            ))
        })?;
        let generator =
            MultisigPublicKey::aggregate(&[generator]).map_err(|err| {
                HeaderError::InvalidBlockSignature(format!(
                    "failed aggregating single key: {err:?}"
                ))
            })?;

        // Verify block signature
        let block_sig = MultisigSignature::from_bytes(header.signature.inner())
            .map_err(|err| {
                HeaderError::InvalidBlockSignature(format!(
                    "invalid block signature bytes: {err:?}"
                ))
            })?;
        generator.verify(&block_sig, &header.hash).map_err(|err| {
            HeaderError::InvalidBlockSignature(format!(
                "invalid block signature: {err:?}"
            ))
        })?;

        Ok(generator)
    }

    /// Performs sanity checks for basic fields of the block header
    async fn verify_block_header_basic_fields(
        &self,
        header: &'a ledger::Header,
        expected_generator: &PublicKeyBytes,
    ) -> Result<(), HeaderError> {
        let generator =
            self.verify_block_generator(header, expected_generator)?;

        if header.version != BLOCK_HEADER_VERSION {
            return Err(HeaderError::UnsupportedVersion);
        }

        if header.hash == [0u8; 32] {
            return Err(HeaderError::EmptyHash);
        }

        if header.height != self.prev_header.height + 1 {
            return Err(HeaderError::MismatchHeight(
                header.height,
                self.prev_header.height,
            ));
        }

        // Ensure rule of minimum block time is addressed
        if header.timestamp < self.prev_header.timestamp + *MINIMUM_BLOCK_TIME {
            return Err(HeaderError::BlockTimeLess);
        }

        // The Emergency Block can only be produced after all iterations in a
        // round have failed. To ensure Dusk (or anyone in possess of the Dusk
        // private key) is not able to shortcircuit a round with an arbitrary
        // block, nodes should only accept an Emergency Block if its timestamp
        // is higher than the maximum time needed to run all round iterations.
        // This guarantees the network has enough time to actually produce a
        // block, if possible.
        if is_emergency_block(header.iteration)
            && header.timestamp
                < self.prev_header.timestamp
                    + MIN_EMERGENCY_BLOCK_TIME.as_secs()
        {
            return Err(HeaderError::BlockTimeLess);
        }

        let local_time = get_current_timestamp();

        if header.timestamp > local_time + MARGIN_TIMESTAMP {
            return Err(HeaderError::BlockTimeHigher(header.timestamp));
        }

        if header.prev_block_hash != self.prev_header.hash {
            return Err(HeaderError::PrevBlockHash);
        }

        // Ensure block is not already in the ledger
        let block_exists = self
            .db
            .read()
            .await
            .view(|db| db.block_exists(&header.hash))
            .map_err(|e| {
                HeaderError::Storage(
                    "error checking Ledger::get_block_exists",
                    e,
                )
            })?;

        if block_exists {
            return Err(HeaderError::BlockExists);
        }

        // Verify seed field
        self.verify_seed_field(header.seed.inner(), &generator)?;

        Ok(())
    }

    fn verify_seed_field(
        &self,
        seed: &[u8; 48],
        pk: &MultisigPublicKey,
    ) -> Result<(), HeaderError> {
        let signature = MultisigSignature::from_bytes(seed).map_err(|err| {
            HeaderError::InvalidSeed(format!(
                "invalid seed signature bytes: {err:?}"
            ))
        })?;

        pk.verify(&signature, self.prev_header.seed.inner())
            .map_err(|err| {
                HeaderError::InvalidSeed(format!("invalid seed: {err:?}"))
            })?;

        Ok(())
    }

    async fn verify_prev_block_cert(
        &self,
        candidate_block: &'a ledger::Header,
    ) -> Result<Vec<Voter>, HeaderError> {
        if self.prev_header.height == 0
            || is_emergency_block(self.prev_header.iteration)
        {
            return Ok(vec![]);
        }

        let prev_block_hash = candidate_block.prev_block_hash;

        let prev_block_seed = self
            .db
            .read()
            .await
            .view(|v| v.block_header(&self.prev_header.prev_block_hash))
            .map_err(|e| {
                HeaderError::Storage(
                    "error checking Ledger::fetch_block_header",
                    e,
                )
            })?
            .ok_or(HeaderError::Generic("Header not found"))
            .map(|h| h.seed)?;

        let voters = verify_att(
            &candidate_block.prev_block_cert,
            self.prev_header.to_consensus_header(),
            prev_block_seed,
            self.provisioners.prev(),
            Some(RatificationResult::Success(Vote::Valid(prev_block_hash))),
        )
        .await?;

        Ok(voters)
    }

    /// Verify the Failed Iterations field in a block.
    ///
    /// Return the number of attested failed iterations. We refer to this number
    /// as Previous Non-Attested Iterations, or PNI
    async fn verify_failed_iterations(
        &self,
        candidate_block: &'a ledger::Header,
    ) -> Result<u8, FailedIterationError> {
        let mut failed_atts = 0u8;

        let att_list = &candidate_block.failed_iterations.att_list;

        if att_list.len() > RELAX_ITERATION_THRESHOLD as usize {
            return Err(FailedIterationError::TooMany(att_list.len()));
        }

        for (iter, att) in att_list.iter().enumerate() {
            if let Some((att, pk)) = att {
                debug!(event = "verify fail attestation", iter);

                let expected_pk = self.provisioners.current().get_generator(
                    iter as u8,
                    self.prev_header.seed,
                    candidate_block.height,
                );

                if pk != &expected_pk {
                    return Err(FailedIterationError::InvalidGenerator(
                        expected_pk,
                    ));
                }

                let mut consensus_header =
                    candidate_block.to_consensus_header();
                consensus_header.iteration = iter as u8;

                verify_att(
                    att,
                    consensus_header,
                    self.prev_header.seed,
                    self.provisioners.current(),
                    Some(RatificationResult::Fail(Vote::default())),
                )
                .await?;

                failed_atts += 1;
            }
        }

        // In case of Emergency Block, which iteration number is u8::MAX, we
        // count failed iterations up to CONSENSUS_MAX_ITER
        let last_iter = cmp::min(candidate_block.iteration, CONSENSUS_MAX_ITER);

        Ok(last_iter - failed_atts)
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
        if is_emergency_iter(fault_header.iteration) {
            return Err(InvalidFault::EmergencyIteration);
        }
        db.read()
            .await
            .view(|db| {
                let prev_header = db
                    .block_header(&fault_header.prev_block_hash)?
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
                let stored_faults = db.faults_by_block(start_height)?;
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
    seed: Seed,
    eligible_provisioners: &Provisioners,
    expected_result: Option<RatificationResult>,
) -> Result<Vec<Voter>, AttestationError> {
    // Check expected result
    if let Some(expected) = expected_result {
        match (att.result, expected) {
            // Both are Success and the inner Valid(Hash) values match
            (
                RatificationResult::Success(Vote::Valid(r_hash)),
                RatificationResult::Success(Vote::Valid(e_hash)),
            ) => {
                if r_hash != e_hash {
                    error!("Invalid Attestation. Expected: Valid({:?}), got: Valid({:?})", hex::encode(e_hash), hex::encode(r_hash));
                    return Err(AttestationError::InvalidHash(e_hash, r_hash));
                }
            }
            // Both are Fail
            (RatificationResult::Fail(_), RatificationResult::Fail(_)) => {}
            // All other mismatches
            _ => {
                error!(
                    "Invalid Attestation. Expected: {:?}, got: {:?}",
                    expected, att.result
                );
                return Err(AttestationError::InvalidResult(
                    att.result, expected,
                ));
            }
        }
    }

    let committee_set = RwLock::new(CommitteeSet::new(eligible_provisioners));
    let vote = att.result.vote();

    // Verify Validation votes
    let validation_voters = verifiers::verify_step_votes(
        &consensus_header,
        vote,
        &att.validation,
        &committee_set,
        seed,
        StepName::Validation,
    )
    .await
    .map_err(|s| AttestationError::InvalidVotes(StepName::Validation, s))?;

    // Verify Ratification votes
    let ratification_voters = verifiers::verify_step_votes(
        &consensus_header,
        vote,
        &att.ratification,
        &committee_set,
        seed,
        StepName::Ratification,
    )
    .await
    .map_err(|s| AttestationError::InvalidVotes(StepName::Ratification, s))?;

    let voters = merge_voters(validation_voters, ratification_voters);
    Ok(voters)
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
