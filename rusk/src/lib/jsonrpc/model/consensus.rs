// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # JSON-RPC Consensus Models
//!
//! Defines data structures representing information related to the consensus
//! process, intended for use in the JSON-RPC API.
//!
//! These structures typically represent the outcomes of consensus steps, such
//! as validation results.
//!
//! ## Key Structures:
//!
//! - [`ValidationResult`]: Represents the outcome of a consensus validation
//!   step, including the quorum type, collective vote, and signature count.
//! - [`QuorumType`]: Indicates the type of quorum reached (e.g., `Valid`,
//!   `Invalid`).
//! - [`VoteType`]: Represents the collective vote outcome (e.g., `Valid` with
//!   block hash, `NoCandidate`).
//!
//! ## Conversions:
//!
//! `From` implementations are provided to convert internal node data types
//! (from `node_data::message::payload`) into these simplified JSON-RPC
//! models.

use node_data::message::payload::{
    QuorumType as NodeQuorumType, ValidationResult as NodeValidationResult,
    Vote as NodeVote,
};
use serde::{Deserialize, Serialize};
use std::convert::From;

/// Represents the type of quorum reached during a consensus validation step.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum QuorumType {
    /// A quorum of votes agreed on a valid candidate block.
    Valid,
    /// A quorum of votes agreed that the candidate block was invalid.
    Invalid,
    /// A quorum of votes agreed that no candidate block was available or
    /// proposed.
    NoCandidate,
}

/// Represents the collective validation vote outcome from a consensus step.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum VoteType {
    /// The collective vote confirmed a valid block.
    /// Contains the 32-byte hash of the validated block, serialized as a
    /// 64-character hex string.
    Valid(String),
    /// The collective vote rejected an invalid block.
    /// Contains the 32-byte hash of the rejected block, serialized as a
    /// 64-character hex string.
    Invalid(String),
    /// The collective vote indicated no candidate block was available.
    NoCandidate,
    /// No quorum was reached for any outcome (e.g., due to timeouts).
    /// Note: This state should generally not appear in stored results, as it
    /// represents a transient failure state during consensus.
    NoQuorum,
}

/// Represents the final outcome of a consensus validation step.
///
/// This includes the type of agreement reached (quorum), the specific vote
/// outcome, and the number of participants who contributed signatures.
///
/// # Examples
///
/// ```
/// use rusk::jsonrpc::model::consensus::{ValidationResult, QuorumType, VoteType};
///
/// let result = ValidationResult {
///     quorum: QuorumType::Valid,
///     vote: VoteType::Valid("block_hash_hex".to_string()),
///     signature_count: 45, // Number of unique signers
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationResult {
    /// The type of quorum agreement reached by the consensus participants.
    pub quorum: QuorumType,
    /// The collective vote outcome based on the quorum.
    pub vote: VoteType,
    /// The number of unique committee members whose signatures contributed to
    /// the aggregated `StepVotes`.
    /// This count is derived from the bitset in the internal `StepVotes`
    /// structure and represents the number of individual participants, not
    /// necessarily the total voting power if weighting is involved.
    /// Serialized as a numeric string.
    #[serde(with = "crate::jsonrpc::model::serde_helper::u64_to_string")]
    pub signature_count: u64,
}

// --- Conversion Implementations ---

/// Converts the node's internal quorum type (`NodeQuorumType`) into the
/// JSON-RPC `QuorumType`.
///
/// # Panics
///
/// This function will panic if the input is `NodeQuorumType::NoQuorum`. This
/// state represents a transient failure during consensus (e.g., timeout) and
/// is not expected to be stored or represented in the final `ValidationResult`
/// exposed via RPC. Encountering it here indicates an unexpected state or
/// misuse.
impl From<NodeQuorumType> for QuorumType {
    fn from(node_quorum: NodeQuorumType) -> Self {
        match node_quorum {
            NodeQuorumType::Valid => QuorumType::Valid,
            NodeQuorumType::Invalid => QuorumType::Invalid,
            NodeQuorumType::NoCandidate => QuorumType::NoCandidate,
            NodeQuorumType::NoQuorum => {
                // This state typically shouldn't be stored or represented in a
                // final ValidationResult model intended for RPC. If
                // encountered, it signifies an unexpected state for this
                // conversion.
                panic!(
                    "Unexpected NodeQuorumType::NoQuorum encountered during conversion to jsonrpc::model::QuorumType"
                );
            }
        }
    }
}

/// Converts the node's internal vote type (`NodeVote`) into the JSON-RPC
/// `VoteType`.
///
/// Block hashes within `Valid` and `Invalid` variants are converted to hex
/// strings.
impl From<&NodeVote> for VoteType {
    fn from(node_vote: &NodeVote) -> Self {
        match node_vote {
            NodeVote::Valid(hash) => VoteType::Valid(hex::encode(hash)),
            NodeVote::Invalid(hash) => VoteType::Invalid(hex::encode(hash)),
            NodeVote::NoCandidate => VoteType::NoCandidate,
            NodeVote::NoQuorum => VoteType::NoQuorum,
        }
    }
}

/// Converts the node's internal validation result
/// (`NodeValidationResult`) into the JSON-RPC `ValidationResult` model.
///
/// The `signature_count` is derived from the number of set bits in the
/// underlying `StepVotes` bitset, representing the count of unique signing
/// participants.
impl From<NodeValidationResult> for ValidationResult {
    fn from(node_result: NodeValidationResult) -> Self {
        // The `signature_count` represents the number of unique committee
        // members whose signatures are included in the aggregate
        // signature stored in `StepVotes`. This is derived from the
        // number of set bits in the `StepVotes.bitset`.
        // It does NOT represent the total weighted vote count if members have
        // varying weights.
        let signature_count =
            node_result.step_votes().bitset.count_ones() as u64;

        // Ensure conversion from NodeVote doesn't panic here if NoQuorum occurs
        // If node_result.vote() can be NoQuorum, handle it gracefully if
        // needed, though the QuorumType conversion might panic first if
        // it's NoQuorum.
        let vote = VoteType::from(node_result.vote());
        // QuorumType::from might panic if node_result.quorum() is NoQuorum.
        let quorum = QuorumType::from(node_result.quorum());

        Self {
            quorum,
            vote,
            signature_count,
        }
    }
}
