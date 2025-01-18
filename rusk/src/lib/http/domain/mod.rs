// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod constants;
pub mod error;
pub(crate) mod factory;
pub mod formats;
pub mod processing;
pub mod types;
pub mod validation;

// Re-export factory methods for creating domain types which are not
// intended to be used directly by the user but only in processing pipelines.
pub(crate) use factory::DomainTypesFactory;

// Helper functions for documentation examples. Not intended for use in
// production code and therefore hidden from the public API.
//
// These functions allow to create types which constructors visibility
// is restricted to the crate. Usually, these types should be created in
// processing pipelines and not directly by the user but for the sake of
// documentation examples, we expose them here.
#[doc(hidden)]
pub mod testing {
    use super::*;
    use crate::http::domain::types::event::Version;
    use crate::http::domain::types::identifier::{
        BlockHash, ContractId, IdentifierBytes, SessionId, TargetIdentifier,
        TransactionHash,
    };
    use crate::http::domain::types::path::{
        LegacyTarget, RuesPath, Target, Topic,
    };
    use crate::http::domain::types::value::RuesValue;
    use bytes::Bytes;

    /// Creates a test block hash identifier.
    pub fn create_test_block_hash() -> TargetIdentifier {
        TargetIdentifier::Block(BlockHash(IdentifierBytes(RuesValue::Binary(
            Bytes::copy_from_slice(&[1u8; 32]),
        ))))
    }

    /// Creates a different test block hash identifier.
    pub fn create_different_block_hash() -> TargetIdentifier {
        TargetIdentifier::Block(BlockHash(IdentifierBytes(RuesValue::Binary(
            Bytes::copy_from_slice(&[2u8; 32]),
        ))))
    }

    /// Creates a test transaction hash identifier.
    pub fn create_test_tx_hash() -> TargetIdentifier {
        TargetIdentifier::Transaction(TransactionHash(IdentifierBytes(
            RuesValue::Binary(Bytes::copy_from_slice(&[3u8; 32])),
        )))
    }

    /// Creates a different test transaction hash identifier.
    pub fn create_different_tx_hash() -> TargetIdentifier {
        TargetIdentifier::Transaction(TransactionHash(IdentifierBytes(
            RuesValue::Binary(Bytes::copy_from_slice(&[4u8; 32])),
        )))
    }

    /// Creates a test contract identifier.
    pub fn create_test_contract_id() -> TargetIdentifier {
        TargetIdentifier::Contract(ContractId(IdentifierBytes(
            RuesValue::Binary(Bytes::from(vec![5u8; 12])),
        )))
    }

    /// Creates a different test contract identifier.
    pub fn create_different_contract_id() -> TargetIdentifier {
        TargetIdentifier::Contract(ContractId(IdentifierBytes(
            RuesValue::Binary(Bytes::from(vec![6u8; 12])),
        )))
    }

    /// Creates a test session identifier.
    pub fn create_test_session_id() -> SessionId {
        SessionId(IdentifierBytes(RuesValue::Binary(Bytes::copy_from_slice(
            &[7u8; 16],
        ))))
    }

    /// Creates a different test session identifier.
    pub fn create_different_session_id() -> SessionId {
        SessionId(IdentifierBytes(RuesValue::Binary(Bytes::copy_from_slice(
            &[8u8; 16],
        ))))
    }

    /// Creates an invalid identifier (non-binary RuesValue).
    pub fn create_invalid_identifier() -> IdentifierBytes {
        IdentifierBytes(RuesValue::Text("not binary".into()))
    }

    /// Creates an invalid session ID
    pub fn create_invalid_session_id() -> SessionId {
        SessionId(create_invalid_identifier())
    }

    /// Creates an invalid block hash
    pub fn create_invalid_block_hash() -> BlockHash {
        BlockHash(create_invalid_identifier())
    }

    /// Creates an invalid transaction hash
    pub fn create_invalid_transaction_hash() -> TransactionHash {
        TransactionHash(create_invalid_identifier())
    }

    /// Creates an invalid contract ID
    pub fn create_invalid_contract_id() -> ContractId {
        ContractId(create_invalid_identifier())
    }

    /// Creates a test version.
    pub fn create_test_version(
        major: u8,
        minor: u8,
        patch: u8,
        pre_release: Option<u8>,
    ) -> Version {
        DomainTypesFactory::create_version(major, minor, patch, pre_release)
    }

    /// Creates a release version (1.0.0)
    pub fn create_release_version() -> Version {
        DomainTypesFactory::create_version(1, 0, 0, None)
    }

    /// Creates a pre-release version (1.0.0-1)
    pub fn create_pre_release_version() -> Version {
        DomainTypesFactory::create_version(1, 0, 0, Some(1))
    }

    /// Creates a test topic.
    pub fn create_test_topic(value: impl Into<String>) -> Topic {
        DomainTypesFactory::create_topic(value)
    }

    /// Creates a test RUES path.
    ///
    /// # Arguments
    /// * `target` - Target type
    /// * `id` - Optional target identifier
    /// * `topic` - Topic string
    pub fn create_test_rues_path(
        target: Target,
        id: Option<TargetIdentifier>,
        topic: impl Into<String>,
    ) -> RuesPath {
        RuesPath::new_modern(
            target,
            id,
            DomainTypesFactory::create_topic(topic),
        )
    }

    /// Creates a test legacy RUES path.
    pub fn create_test_legacy_path(
        target: LegacyTarget,
        topic: impl Into<String>,
    ) -> RuesPath {
        RuesPath::new_legacy(target, DomainTypesFactory::create_topic(topic))
    }
}
