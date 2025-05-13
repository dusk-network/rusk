// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use rusk::jsonrpc::infrastructure::archive::ArchiveAdapter;
use rusk::jsonrpc::infrastructure::error::ArchiveError;
use rusk::jsonrpc::model;
use tempfile::tempdir;

use std::collections::HashMap;
use std::fmt::Debug;

/// A mock implementation of `ArchiveAdapter` for testing purposes.
#[derive(Debug, Clone, Default)]
pub struct MockArchiveAdapter {
    /// Mock storage for transaction groups keyed by memo bytes (as Vec<u8>).
    pub txs_by_memo: HashMap<Vec<u8>, Vec<model::archive::MoonlightEventGroup>>,
    /// Mock storage for last archived block (height, hash).
    pub last_archived_block: Option<(u64, String)>,
    /// Mock storage for events keyed by hex block hash.
    pub events_by_hash: HashMap<String, Vec<model::archive::ArchivedEvent>>,
    /// Mock storage for events keyed by block height.
    pub events_by_height: HashMap<u64, Vec<model::archive::ArchivedEvent>>,
    /// Mock storage for finalized events keyed by contract ID string.
    pub finalized_events_by_contract:
        HashMap<String, Vec<model::archive::ArchivedEvent>>,
    /// Mock mapping from input height to the next height with a phoenix tx.
    pub next_phoenix_height: HashMap<u64, Option<u64>>,
    /// Mock storage for moonlight history keyed by bs58 public key.
    pub moonlight_history:
        HashMap<String, Vec<model::archive::MoonlightEventGroup>>,
    /// Optional error to return from all methods.
    pub force_error: Option<ArchiveError>,
}

#[async_trait::async_trait]
impl ArchiveAdapter for MockArchiveAdapter {
    /// Returns predefined transaction groups based on memo, or `force_error` if
    /// set.
    async fn get_moonlight_txs_by_memo(
        &self,
        memo: Vec<u8>,
    ) -> Result<Option<Vec<model::archive::MoonlightEventGroup>>, ArchiveError>
    {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        // Get by Vec<u8> key
        Ok(self.txs_by_memo.get(&memo).cloned())
    }

    /// Returns the predefined `last_archived_block`, or `force_error` if set.
    async fn get_last_archived_block(
        &self,
    ) -> Result<(u64, String), ArchiveError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        // Return Option<(u64, String)> or default
        self.last_archived_block.clone().ok_or_else(|| {
            ArchiveError::NotFound("Mock last archived block not set".into())
        })
    }

    /// Returns predefined events based on hash, or `force_error` if set.
    async fn get_block_events_by_hash(
        &self,
        hex_block_hash: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        Ok(self
            .events_by_hash
            .get(hex_block_hash)
            .cloned()
            .unwrap_or_default())
    }

    /// Returns predefined events based on height, or `force_error` if set.
    async fn get_block_events_by_height(
        &self,
        block_height: u64,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        Ok(self
            .events_by_height
            .get(&block_height)
            .cloned()
            .unwrap_or_default())
    }

    /// Returns events from the latest mock height, or `force_error` if set.
    async fn get_latest_block_events(
        &self,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        let (height, _) = self.get_last_archived_block().await?;
        self.get_block_events_by_height(height).await
    }

    /// Returns predefined finalized events based on contract ID, or
    /// `force_error` if set.
    async fn get_contract_finalized_events(
        &self,
        contract_id: &str,
    ) -> Result<Vec<model::archive::ArchivedEvent>, ArchiveError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        Ok(self
            .finalized_events_by_contract
            .get(contract_id)
            .cloned()
            .unwrap_or_default())
    }

    /// Returns predefined next phoenix height, or `force_error` if set.
    async fn get_next_block_with_phoenix_transaction(
        &self,
        block_height: u64,
    ) -> Result<Option<u64>, ArchiveError> {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        // Return predefined Option<u64> or default None
        Ok(self
            .next_phoenix_height
            .get(&block_height)
            .cloned()
            .flatten())
    }

    /// Returns predefined moonlight history based on public key, or
    /// `force_error` if set.
    async fn get_moonlight_transaction_history(
        &self,
        pk_bs58: String,
        _ord: Option<model::archive::Order>,
        _from_block: Option<u64>,
        _to_block: Option<u64>,
    ) -> Result<Option<Vec<model::archive::MoonlightEventGroup>>, ArchiveError>
    {
        if let Some(err) = self.force_error.clone() {
            return Err(err);
        }
        // Ignore order/range parameters in mock
        Ok(self.moonlight_history.get(&pk_bs58).cloned())
    }
}

/// Helper to setup a temporary archive
#[cfg(feature = "archive")]
pub(crate) async fn setup_test_archive(
) -> (tempfile::TempDir, ::node::archive::Archive) {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let archive =
        ::node::archive::Archive::create_or_open(temp_dir.path()).await;
    (temp_dir, archive)
}
