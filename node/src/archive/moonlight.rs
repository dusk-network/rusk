// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use core::result::Result as CoreResult;
use dusk_bytes::Serializable;
use execution_core::signatures::bls::PublicKey as AccountPublicKey;
use node_data::events::contract::{ContractTxEvent, TxHash};
use rocksdb::{
    BlockBasedOptions, ColumnFamily, ColumnFamilyDescriptor, LogLevel,
    OptimisticTransactionDB, Options,
};
use tracing::{debug, info, warn};

use crate::archive::transformer::{self, MoonlightTxEvents};
use crate::archive::{Archive, ArchiveOptions};

/// Subfolder containing the moonlight database.
const MOONLIGHT_DB_FOLDER_NAME: &str = "moonlight.db";

/*
 * Column family names.
 */
// Moonlight TxHash to ContractMoonlightEvents mapping
const CF_MTXHASH_MEVENTS: &str = "cf_mtxhash_mevents";
// AccountPublicKey to Moonlight Inflow TxHashes mapping
const CF_M_INFLOW_ADDRESS_TXHASH: &str = "cf_m_inflow_address_txhash";
// AccountPublicKey to Moonlight Outflow TxHashes mapping
const CF_M_OUTFLOW_ADDRESS_TXHASH: &str = "cf_m_outflow_address_txhash";
// Memo to Moonlight TxHashes mapping (in- & outlfows)
const CF_M_MEMO_TXHASH: &str = "cf_m_memo_txhash";

pub struct MoonlightFlows {
    pub inflows: Option<Vec<TxHash>>,
    pub outflows: Option<Vec<TxHash>>,
}

impl Archive {
    /// Create or open the moonlight database.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the archive folder.
    /// * `archive_opts` - The options for the archive.
    pub(super) async fn create_or_open_moonlight_db<
        P: AsRef<Path> + std::fmt::Debug,
    >(
        path: P,
        archive_opts: ArchiveOptions,
    ) -> Arc<OptimisticTransactionDB> {
        info!("Opening moonlight db in {path:?}, {archive_opts:?} ");

        let path = path.as_ref().join(MOONLIGHT_DB_FOLDER_NAME);

        let mut rocksdb_opts = Options::default();
        rocksdb_opts.create_if_missing(true);
        rocksdb_opts.create_missing_column_families(true);
        rocksdb_opts.set_level_compaction_dynamic_level_bytes(true);
        rocksdb_opts.set_write_buffer_size(
            archive_opts.events_cf_max_write_buffer_size,
        );

        if archive_opts.enable_debug {
            rocksdb_opts.set_log_level(LogLevel::Info);
            rocksdb_opts.set_dump_malloc_stats(true);
            rocksdb_opts.enable_statistics();
        }

        if archive_opts.events_cf_disable_block_cache {
            let mut block_opts = BlockBasedOptions::default();
            block_opts.disable_cache();
            rocksdb_opts.set_block_based_table_factory(&block_opts);
        }

        let cfs = vec![
            ColumnFamilyDescriptor::new(
                CF_MTXHASH_MEVENTS,
                rocksdb_opts.clone(),
            ),
            ColumnFamilyDescriptor::new(
                CF_M_INFLOW_ADDRESS_TXHASH,
                rocksdb_opts.clone(),
            ),
            ColumnFamilyDescriptor::new(
                CF_M_OUTFLOW_ADDRESS_TXHASH,
                rocksdb_opts.clone(),
            ),
            ColumnFamilyDescriptor::new(CF_M_MEMO_TXHASH, rocksdb_opts.clone()),
        ];

        Arc::new(
            OptimisticTransactionDB::open_cf_descriptors(
                &rocksdb_opts,
                path,
                cfs,
            )
            .expect("should be a valid database in {path}"),
        )
    }

    fn cf_txhash_moonlight_events(&self) -> Result<&ColumnFamily> {
        self.moonlight_db
            .cf_handle(CF_MTXHASH_MEVENTS)
            .ok_or(anyhow!("Column family not found"))
    }

    fn cf_m_inflow_address_txhash(&self) -> Result<&ColumnFamily> {
        self.moonlight_db
            .cf_handle(CF_M_INFLOW_ADDRESS_TXHASH)
            .ok_or(anyhow!("Column family not found"))
    }

    fn cf_m_outflow_address_txhash(&self) -> Result<&ColumnFamily> {
        self.moonlight_db
            .cf_handle(CF_M_OUTFLOW_ADDRESS_TXHASH)
            .ok_or(anyhow!("Column family not found"))
    }

    fn cf_memo_txhash(&self) -> Result<&ColumnFamily> {
        self.moonlight_db
            .cf_handle(CF_M_MEMO_TXHASH)
            .ok_or(anyhow!("Column family not found"))
    }

    /// Transform & Load moonlight related events into the moonlight database.
    ///
    /// # Arguments
    ///
    /// * `block_events` - All contract events from a finalized block.
    pub(super) fn tl_moonlight(
        &self,
        block_events: Vec<ContractTxEvent>,
        block_height: u64,
    ) -> Result<()> {
        debug!("Loading moonlight transaction events into the moonlight db");

        let transformer::TransormerResult {
            address_outflow_mappings,
            address_inflow_mappings,
            memo_mappings,
            moonlight_tx_groups,
        } = transformer::group_by_origins_filter_and_convert(
            block_events,
            block_height,
        );

        debug!("Found {} moonlight transactions", moonlight_tx_groups.len());

        let address_inflow_mappings =
            util::check_duplicates(address_inflow_mappings);
        let address_outflow_mappings =
            util::check_duplicates(address_outflow_mappings);

        for mapping in address_inflow_mappings {
            let (pk, tx_hash) = mapping;
            self.update_inflow_address_txhash(pk, tx_hash)?;
        }

        for mapping in address_outflow_mappings {
            let (pk, tx_hash) = mapping;
            self.update_outflow_address_txhash(pk, tx_hash)?;
        }

        for moonlight_group in moonlight_tx_groups {
            self.insert_txhash_events(
                *moonlight_group.origin(),
                moonlight_group,
            )?;
        }

        for memo_mapping in memo_mappings {
            let (memo, tx_hash) = memo_mapping;
            self.update_memo_txhash(memo, tx_hash)?;
        }

        Ok(())
    }

    /// Insert or update an AccountPublicKey to TxHash mapping for inflows.
    fn update_inflow_address_txhash(
        &self,
        pk: AccountPublicKey,
        tx_hash: TxHash,
    ) -> Result<()> {
        let cf_inflow = self.cf_m_inflow_address_txhash()?;
        let key = pk.to_bytes();

        self.append_txhash(cf_inflow, &key, tx_hash)
    }

    /// Insert or update an AccountPublicKey to TxHash mapping for outflows.
    fn update_outflow_address_txhash(
        &self,
        pk: AccountPublicKey,
        tx_hash: TxHash,
    ) -> Result<()> {
        let cf_outflow = self.cf_m_outflow_address_txhash()?;
        let key = pk.to_bytes();

        self.append_txhash(cf_outflow, &key, tx_hash)
    }

    /// Insert or update a Memo to TxHash mapping.
    fn update_memo_txhash(&self, memo: Vec<u8>, tx_hash: TxHash) -> Result<()> {
        let cf_memo = self.cf_memo_txhash()?;
        let key = memo;

        self.append_txhash(cf_memo, &key, tx_hash)
    }

    fn append_txhash(
        &self,
        cf: &ColumnFamily,
        key: &[u8],
        tx_hash: TxHash,
    ) -> Result<()> {
        let txn = self.moonlight_db.transaction();

        let existing_tx_hashes = txn.get_cf(cf, key)?;

        if let Some(tx_hashes) = existing_tx_hashes {
            let mut tx_hashes =
                serde_json::from_slice::<Vec<TxHash>>(&tx_hashes)?;

            // Append the new TxHash to the existing tx hashes
            tx_hashes.push(tx_hash);

            // Put the updated tx hashes back into the CF
            txn.put_cf(cf, key, serde_json::to_vec(&tx_hashes)?)?;

            txn.commit()?;

            Ok(())
        } else {
            // Serialize the TxHash and put it into the CF
            txn.put_cf(cf, key, serde_json::to_vec(&vec![tx_hash])?)?;

            txn.commit()?;

            Ok(())
        }
    }

    /// Insert new moonlight event(s) for a TxHash.
    fn insert_txhash_events(
        &self,
        tx_hash: TxHash,
        events: MoonlightTxEvents,
    ) -> Result<()> {
        let txn = self.moonlight_db.transaction();
        let cf = self.cf_txhash_moonlight_events()?;

        // Check if the TxHash already exists in the database
        // If it does, return false, to not overwrite the existing events
        if txn.get_cf(cf, tx_hash)?.is_some() {
            return Err(anyhow!(
                "TxHash already exists. This should not happen"
            ));
        }

        // Serialize the events and put them into the database
        let v = serde_json::to_vec(&events)?;

        txn.put_cf(cf, tx_hash, v)?;

        txn.commit()?;

        Ok(())
    }

    /// Get the full moonlight transaction history of a given AccountPublicKey.
    ///
    /// Every MoonlightTxEvents is associated with a TxHash.
    /// The underlying Vec<ContractEvent> contains at least one event that
    /// relates to a moonlight in- or outflow.
    ///
    /// This can be a "moonlight" event or
    /// a "withdraw", "mint" or "convert" event, where there is a Moonlight
    /// address as WithdrawReceiver
    pub fn moonlight_txs_by_pk(
        &self,
        pk: AccountPublicKey,
    ) -> Result<Option<Vec<MoonlightTxEvents>>> {
        let MoonlightFlows { inflows, outflows } =
            self.moonlight_flows(Some(pk), Some(pk))?;

        // merge inflows and outflows
        let mut tx_hashes = Vec::new();
        if let Some(inflows) = inflows {
            tx_hashes.extend(inflows);
        }
        if let Some(outflows) = outflows {
            tx_hashes.extend(outflows);
        }

        if tx_hashes.is_empty() {
            Ok(None)
        } else {
            self.moonlight_txs_by_txhash(tx_hashes)
        }
    }

    /// Get a vector of Vec<MoonlightTxEvents> for a given memo.
    ///
    /// Clients are advised to check if the Vec is > 1, as memos are not unique.
    pub fn moonlight_txs_by_memo(
        &self,
        memo: Vec<u8>,
    ) -> Result<Option<Vec<MoonlightTxEvents>>> {
        if let Some(tx_hashes) = self.get_memo_txhashes(memo)? {
            self.moonlight_txs_by_txhash(tx_hashes)
        } else {
            Ok(None)
        }
    }

    /// Get a vector of Vec<MoonlightTxEvents> for a given list of TxHash.
    pub fn moonlight_txs_by_txhash(
        &self,
        tx_hashes: Vec<TxHash>,
    ) -> Result<Option<Vec<MoonlightTxEvents>>> {
        let multi_get = self.multi_get_moonlight_events(tx_hashes)?;

        let mut deserialized_events = Vec::with_capacity(multi_get.len());

        for serialized_event in multi_get {
            if let Ok(Some(e)) = serialized_event {
                deserialized_events
                    .push(serde_json::from_slice::<MoonlightTxEvents>(&e)?);
            } else {
                warn!("Serialized moonlight event not found");
                continue;
            }
        }

        if deserialized_events.is_empty() {
            Ok(None)
        } else {
            Ok(Some(deserialized_events))
        }
    }

    pub fn raw_moonlight_txs_by_txhash(
        &self,
        tx_hashes: Vec<TxHash>,
    ) -> Result<Option<Vec<Vec<u8>>>> {
        let multi_get = self.multi_get_moonlight_events(tx_hashes)?;

        // Convert the Vec<Result<Option<Vec<u8>>> into a Vec<Vec<u8>> by
        // throwing away the errors and None values
        let mut raw = Vec::with_capacity(multi_get.len());

        for serialized_event in multi_get {
            if let Ok(Some(e)) = serialized_event {
                raw.push(e);
            } else {
                warn!("Serialized moonlight event not found");
                continue;
            }
        }

        Ok(Some(raw))
    }

    /// Get a vector of TxHash that relate to moonlight
    /// in- or outflows for a given AccountPublicKey.
    pub fn moonlight_flows(
        &self,
        sender: Option<AccountPublicKey>,
        receiver: Option<AccountPublicKey>,
    ) -> Result<MoonlightFlows> {
        let mut outflows = None;
        let mut inflows = None;

        match (sender, receiver) {
            (None, Some(receiver)) => {
                inflows = self.get_moonlight_inflow_txhashes(receiver)?
            }
            (Some(sender), None) => {
                outflows = self.get_moonlight_outflow_txhashes(sender)?
            }
            (Some(sender), Some(receiver)) => {
                inflows = self.get_moonlight_inflow_txhashes(receiver)?;
                outflows = self.get_moonlight_outflow_txhashes(sender)?;
            }
            _ => return Err(anyhow!("No sender or receiver provided")),
        }

        Ok(MoonlightFlows { inflows, outflows })
    }

    fn get_moonlight_outflow_txhashes(
        &self,
        sender: AccountPublicKey,
    ) -> Result<Option<Vec<TxHash>>> {
        if let Some(tx_hashes) = self
            .moonlight_db
            .get_cf(self.cf_m_outflow_address_txhash()?, sender.to_bytes())?
        {
            Ok(Some(serde_json::from_slice::<Vec<TxHash>>(&tx_hashes)?))
        } else {
            Ok(None)
        }
    }

    fn get_moonlight_inflow_txhashes(
        &self,
        receiver: AccountPublicKey,
    ) -> Result<Option<Vec<TxHash>>> {
        if let Some(tx_hashes) = self
            .moonlight_db
            .get_cf(self.cf_m_inflow_address_txhash()?, receiver.to_bytes())?
        {
            Ok(Some(serde_json::from_slice::<Vec<TxHash>>(&tx_hashes)?))
        } else {
            Ok(None)
        }
    }

    /// Get a vector of TxHash that relate to moonlight
    /// in- or outflows for a given memo.
    pub fn get_memo_txhashes(
        &self,
        memo: Vec<u8>,
    ) -> Result<Option<Vec<TxHash>>> {
        if let Some(tx_hashes) =
            self.moonlight_db.get_cf(self.cf_memo_txhash()?, memo)?
        {
            Ok(Some(serde_json::from_slice::<Vec<TxHash>>(&tx_hashes)?))
        } else {
            Ok(None)
        }
    }

    /// Get MoonlightTxEvents for a given TxHash.
    pub fn get_moonlight_events(
        &self,
        tx_hashes: TxHash,
    ) -> Result<Option<Vec<u8>>> {
        if let Some(events) = self
            .moonlight_db
            .get_cf(self.cf_txhash_moonlight_events()?, tx_hashes)?
        {
            Ok(Some(events))
        } else {
            Ok(None)
        }
    }

    fn tx_hashes_multiget_key_tuple(
        &self,
        tx_hashes: Vec<TxHash>,
    ) -> Result<Vec<(&ColumnFamily, [u8; 32])>> {
        let mut keys = Vec::with_capacity(tx_hashes.len());
        let cf = self.cf_txhash_moonlight_events()?;

        for tx_hash in tx_hashes {
            keys.push((cf, tx_hash));
        }

        Ok(keys)
    }

    /// Get multiple MoonlightTxEvents for a given list of TxHash.
    fn multi_get_moonlight_events(
        &self,
        tx_hashes: Vec<TxHash>,
    ) -> Result<Vec<CoreResult<Option<Vec<u8>>, rocksdb::Error>>> {
        let keys = self.tx_hashes_multiget_key_tuple(tx_hashes)?;
        Ok(self.moonlight_db.multi_get_cf(keys))
    }
}

mod util {
    use super::{AccountPublicKey, Serializable, TxHash};
    use tracing::warn;

    /// Check and remove duplicates from a list of address mappings.
    pub(super) fn check_duplicates(
        address_mappings: Vec<(AccountPublicKey, TxHash)>,
    ) -> Vec<(AccountPublicKey, TxHash)> {
        // Check for duplicates
        let len = address_mappings.len();
        let mut seen = std::collections::HashSet::new();
        let mut deduped = Vec::new();

        for (pk, txh) in address_mappings {
            if seen.insert((pk.to_bytes(), txh)) {
                deduped.push((pk, txh));
            }
        }

        if len != deduped.len() {
            warn!("Found duplicates in address mappings for moonlight transactions. Duplicates have been removed. This is a bug.");
        }

        deduped
    }
}

#[cfg(test)]
mod tests {
    use super::transformer::{
        group_by_origins_filter_and_convert, TransormerResult,
    };
    use super::*;
    use execution_core::transfer::withdraw::WithdrawReceiver;
    use execution_core::transfer::{
        ConvertEvent, DepositEvent, MoonlightTransactionEvent, WithdrawEvent,
    };
    use execution_core::{ContractId, CONTRACT_ID_BYTES};
    use node_data::events::contract::{
        ContractEvent, WrappedContractId, TX_HASH_BYTES,
    };
    use rand::{distributions::Alphanumeric, Rng};
    use std::env;
    use std::path::PathBuf;

    // Construct a random test directory path in the temp folder of the OS
    fn test_dir() -> PathBuf {
        let mut test_dir = "archive-rocksdb-test-".to_owned();
        let rand_string: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(20)
            .map(char::from)
            .collect();
        test_dir.push_str(&rand_string);

        env::temp_dir().join(test_dir)
    }

    fn dummy_data(topic: &str) -> ContractTxEvent {
        ContractTxEvent {
            event: ContractEvent {
                target: WrappedContractId(ContractId::from_bytes(
                    [0; CONTRACT_ID_BYTES],
                )),
                topic: topic.to_owned(),
                data: vec![1, 6, 1, 8],
            },
            origin: Some([0; 32]),
        }
    }

    fn phoenix_event() -> ContractTxEvent {
        let fake_phoenix_tx_event_data = vec![0, 0, 0, 0, 0];

        ContractTxEvent {
            event: ContractEvent {
                target: WrappedContractId(
                    execution_core::transfer::TRANSFER_CONTRACT,
                ),
                topic: "phoenix".to_string(),
                data: rkyv::to_bytes::<_, 256>(&fake_phoenix_tx_event_data)
                    .unwrap()
                    .to_vec(),
            },
            origin: Some([0; 32]),
        }
    }

    fn convert_event_moonlight() -> ContractTxEvent {
        let convert_event = ConvertEvent {
            sender: None,
            value: 500,
            receiver: WithdrawReceiver::Moonlight(AccountPublicKey::default()),
        };

        ContractTxEvent {
            event: ContractEvent {
                target: WrappedContractId(
                    execution_core::transfer::TRANSFER_CONTRACT,
                ),
                topic: "convert".to_string(),
                data: rkyv::to_bytes::<_, 256>(&convert_event)
                    .unwrap()
                    .to_vec(),
            },
            origin: Some([1; 32]),
        }
    }

    fn moonlight_event(
        origin: [u8; 32],
        receiver: Option<AccountPublicKey>,
        memo: Vec<u8>,
    ) -> ContractTxEvent {
        let moonlight_tx_event = MoonlightTransactionEvent {
            sender: AccountPublicKey::default(),
            receiver,
            value: 500,
            memo,
            gas_spent: 500,
        };

        ContractTxEvent {
            event: ContractEvent {
                target: WrappedContractId(
                    execution_core::transfer::TRANSFER_CONTRACT,
                ),
                topic: "moonlight".to_string(),
                data: rkyv::to_bytes::<_, 256>(&moonlight_tx_event)
                    .unwrap()
                    .to_vec(),
            },
            origin: Some(origin),
        }
    }

    fn withdraw_event_moonlight() -> ContractTxEvent {
        let withdraw_event = WithdrawEvent {
            sender: ContractId::from_bytes([5; CONTRACT_ID_BYTES]),
            value: 100,
            receiver: WithdrawReceiver::Moonlight(AccountPublicKey::default()),
        };

        ContractTxEvent {
            event: ContractEvent {
                target: WrappedContractId(
                    execution_core::transfer::TRANSFER_CONTRACT,
                ),
                topic: "withdraw".to_string(),
                data: rkyv::to_bytes::<_, 256>(&withdraw_event)
                    .unwrap()
                    .to_vec(),
            },
            origin: Some([3; 32]),
        }
    }

    fn deposit_event_moonlight(origin: [u8; 32]) -> ContractTxEvent {
        let deposit_event = DepositEvent {
            sender: Some(AccountPublicKey::default()),
            value: 100,
            receiver: ContractId::from_bytes([5; 32]),
        };

        ContractTxEvent {
            event: ContractEvent {
                target: WrappedContractId(
                    execution_core::transfer::TRANSFER_CONTRACT,
                ),
                topic: "deposit".to_string(),
                data: rkyv::to_bytes::<_, 256>(&deposit_event)
                    .unwrap()
                    .to_vec(),
            },
            origin: Some(origin),
        }
    }

    fn deposit_event_phoenix() -> ContractTxEvent {
        let deposit_event = DepositEvent {
            sender: None,
            value: 100,
            receiver: ContractId::from_bytes([5; 32]),
        };

        ContractTxEvent {
            event: ContractEvent {
                target: WrappedContractId(
                    execution_core::transfer::TRANSFER_CONTRACT,
                ),
                topic: "deposit".to_string(),
                data: rkyv::to_bytes::<_, 256>(&deposit_event)
                    .unwrap()
                    .to_vec(),
            },
            origin: Some([5; 32]),
        }
    }

    fn block_events() -> Vec<ContractTxEvent> {
        vec![
            // should not count
            phoenix_event(),
            dummy_data("dummy"),
            dummy_data("dummy2"),
            dummy_data("moonlight"),
            // should not count & always appear together with a
            // MoonlightTransactionEvent
            deposit_event_moonlight([4; 32]),
            // should count (5 in total)
            convert_event_moonlight(),
            moonlight_event(
                [2; 32],
                Some(AccountPublicKey::default()),
                vec![0, 1, 1, 0],
            ),
            moonlight_event(
                [9; 32],
                Some(AccountPublicKey::default()),
                vec![0, 1, 1, 0],
            ),
            withdraw_event_moonlight(),
            // belongs together with deposit_event_phoenix
            moonlight_event([5; 32], None, vec![0, 1, 1, 0]),
            deposit_event_phoenix(),
        ]
    }

    fn memo_txs() -> Vec<ContractTxEvent> {
        vec![
            moonlight_event([0; 32], None, vec![0, 1, 1, 0]),
            moonlight_event([1; 32], None, vec![0, 1, 1, 0]),
            moonlight_event([2; 32], None, vec![0, 1, 1, 0]),
            moonlight_event([3; 32], None, vec![0, 1, 1, 0]),
            moonlight_event([4; 32], None, vec![1, 1, 1, 1]),
        ]
    }

    #[tokio::test]
    async fn test_event_transformer() {
        let block_events = block_events();

        let TransormerResult {
            address_outflow_mappings,
            address_inflow_mappings,
            memo_mappings,
            moonlight_tx_groups,
        } = group_by_origins_filter_and_convert(block_events, 1);

        assert_eq!(address_outflow_mappings.len(), 3);
        assert_eq!(address_inflow_mappings.len(), 2);
        assert_eq!(memo_mappings.len(), 3);
        // 5 moonlight groups means 5 transactions containing moonlight related
        // events
        assert_eq!(moonlight_tx_groups.len(), 5);
    }

    #[tokio::test]
    async fn test_tl_moonlight_and_fetch() {
        let path = test_dir();
        let archive = Archive::create_or_open(path).await;

        let pk = AccountPublicKey::default();
        assert!(archive.moonlight_txs_by_pk(pk).unwrap().is_none());

        let block_events = block_events();

        // Store block events in the archive
        archive.tl_moonlight(block_events.clone(), 1).unwrap();

        let MoonlightFlows { inflows, outflows } =
            archive.moonlight_flows(Some(pk), Some(pk)).unwrap();
        let fetched_tx_hashes = inflows
            .unwrap()
            .into_iter()
            .chain(outflows.unwrap())
            .collect::<Vec<TxHash>>();

        let fetched_events = archive.moonlight_txs_by_pk(pk).unwrap().unwrap();

        assert_eq!(fetched_tx_hashes.len(), 5);

        for moonlight_events in fetched_events {
            assert_eq!(moonlight_events.block_height(), 1);

            match moonlight_events.origin().as_ref() {
                [1, 1, ..] => {
                    assert_eq!(moonlight_events.events().len(), 1);

                    assert_eq!(moonlight_events.events()[0].topic, "convert");
                }
                [2, 2, ..] => {
                    assert_eq!(moonlight_events.events().len(), 1);
                    assert_eq!(moonlight_events.events()[0].topic, "moonlight");
                }
                [3, 3, ..] => {
                    assert_eq!(moonlight_events.events().len(), 1);
                    assert_eq!(moonlight_events.events()[0].topic, "withdraw");
                }
                [5, 5, ..] => {
                    assert_eq!(moonlight_events.events().len(), 2);
                    assert_eq!(moonlight_events.events()[0].topic, "moonlight");
                    assert_eq!(moonlight_events.events()[1].topic, "deposit");
                }
                [9, 9, ..] => {
                    assert_eq!(moonlight_events.events().len(), 1);
                    assert_eq!(moonlight_events.events()[0].topic, "moonlight");
                }
                _ => panic!("unexpected TxHash"),
            }
        }
    }

    #[tokio::test]
    async fn test_tl_moonlight_and_fetch_single() {
        let path = test_dir();
        let archive = Archive::create_or_open(path).await;
        let pk = AccountPublicKey::default();
        assert!(archive.moonlight_txs_by_pk(pk).unwrap().is_none());

        let block_events = block_events();
        archive.tl_moonlight(block_events.clone(), 1).unwrap();

        let MoonlightFlows { inflows, outflows } =
            archive.moonlight_flows(Some(pk), Some(pk)).unwrap();
        let fetched_tx_hashes = inflows
            .unwrap()
            .into_iter()
            .chain(outflows.unwrap())
            .collect::<Vec<TxHash>>();

        let fetched_events_by_tx_hash =
            serde_json::from_slice::<MoonlightTxEvents>(
                &archive
                    .get_moonlight_events(fetched_tx_hashes[0])
                    .unwrap()
                    .unwrap(),
            )
            .unwrap();

        assert_eq!(fetched_events_by_tx_hash.block_height(), 1);

        assert_eq!(fetched_events_by_tx_hash.events().len(), 1);
        assert_eq!(fetched_events_by_tx_hash.events()[0].topic, "convert");
    }

    #[tokio::test]
    async fn test_tl_memo_and_fetch_single() {
        let path = test_dir();
        let archive = Archive::create_or_open(path).await;

        let block_events = memo_txs();
        archive.tl_moonlight(block_events.clone(), 1).unwrap();

        let fetched_tx1 = archive
            .moonlight_txs_by_memo(vec![1, 1, 1, 1])
            .unwrap()
            .unwrap();
        assert_eq!(fetched_tx1.len(), 1);
        assert_eq!(fetched_tx1[0].origin(), &[4; 32]);
        fetched_tx1[0].events().iter().for_each(|e| {
            assert_eq!(e.topic, "moonlight");
            assert_eq!(
                e.target,
                WrappedContractId(execution_core::transfer::TRANSFER_CONTRACT)
            );

            let moonlight_event =
                rkyv::from_bytes::<MoonlightTransactionEvent>(&e.data).unwrap();
            assert_eq!(moonlight_event.memo, vec![1, 1, 1, 1]);
            assert!(moonlight_event.receiver.is_none());
            assert_eq!(moonlight_event.sender, AccountPublicKey::default());
        });

        let fetched_tx3 = archive
            .moonlight_txs_by_memo(vec![0, 1, 1, 0])
            .unwrap()
            .unwrap();
        assert_eq!(fetched_tx3.len(), 4);

        for (i, fetched_tx) in fetched_tx3.iter().enumerate() {
            assert_eq!(fetched_tx.origin(), &[i as u8; TX_HASH_BYTES]);
            fetched_tx.events().iter().for_each(|e| {
                assert_eq!(e.topic, "moonlight");
                assert_eq!(
                    e.target,
                    WrappedContractId(
                        execution_core::transfer::TRANSFER_CONTRACT
                    )
                );

                let moonlight_event =
                    rkyv::from_bytes::<MoonlightTransactionEvent>(&e.data)
                        .unwrap();
                assert_eq!(moonlight_event.memo, vec![0, 1, 1, 0]);
                assert!(moonlight_event.receiver.is_none());
                assert_eq!(moonlight_event.sender, AccountPublicKey::default());
            });
        }
    }
}
