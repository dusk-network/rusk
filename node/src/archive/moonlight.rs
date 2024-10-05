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
use node_data::events::contract::ContractEvent;
use node_data::events::contract::{ContractTxEvent, TxHash};
use rocksdb::{
    BlockBasedOptions, ColumnFamily, ColumnFamilyDescriptor, DBPinnableSlice,
    LogLevel, OptimisticTransactionDB, Options,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::archive::transformer::{
    self, MoonlightTx, MoonlightTxEvents, MoonlightTxMapping,
};
use crate::archive::{Archive, ArchiveOptions};

/// Subfolder containing the moonlight database.
const MOONLIGHT_DB_FOLDER_NAME: &str = "moonlight.db";

/// Default max count for moonlight transactions returned.
const DEFAULT_MAX_COUNT: usize = 1000;

/*
 * Column family names.
 */
// Moonlight TxHash to MoonlightTxEvents mapping
const CF_MTXHASH_MEVENTS: &str = "cf_mtxhash_mevents";
// AccountPublicKey to Inflow MoonlightTx mapping
const CF_M_INFLOW_ADDRESS_TX: &str = "cf_m_inflow_address_tx";
// AccountPublicKey to Outflow MoonlightTx mapping
const CF_M_OUTFLOW_ADDRESS_TX: &str = "cf_m_outflow_address_tx";
// Memo to MoonlightTx mapping (in- & outlfows)
const CF_M_MEMO_TX: &str = "cf_m_memo_tx";

pub struct MoonlightFlows {
    pub inflows: Option<Vec<MoonlightTx>>,
    pub outflows: Option<Vec<MoonlightTx>>,
}

/// Group of Events that belong to a single Moonlight transaction.
#[serde_with::serde_as]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoonlightGroup {
    events: Vec<ContractEvent>,
    #[serde_as(as = "serde_with::hex::Hex")]
    origin: TxHash,
    block_height: u64,
}

impl MoonlightGroup {
    pub fn events(&self) -> &[ContractEvent] {
        &self.events
    }

    pub fn origin(&self) -> &TxHash {
        &self.origin
    }

    pub fn block_height(&self) -> u64 {
        self.block_height
    }
}

impl Archive {
    /// Create or open the moonlight database.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the archive folder.
    /// * `archive_opts` - The options for the archive.
    pub(super) fn create_or_open_moonlight_db<
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
                CF_M_INFLOW_ADDRESS_TX,
                rocksdb_opts.clone(),
            ),
            ColumnFamilyDescriptor::new(
                CF_M_OUTFLOW_ADDRESS_TX,
                rocksdb_opts.clone(),
            ),
            ColumnFamilyDescriptor::new(CF_M_MEMO_TX, rocksdb_opts.clone()),
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

    fn cf_m_inflow_address_tx(&self) -> Result<&ColumnFamily> {
        self.moonlight_db
            .cf_handle(CF_M_INFLOW_ADDRESS_TX)
            .ok_or(anyhow!("Column family not found"))
    }

    fn cf_m_outflow_address_tx(&self) -> Result<&ColumnFamily> {
        self.moonlight_db
            .cf_handle(CF_M_OUTFLOW_ADDRESS_TX)
            .ok_or(anyhow!("Column family not found"))
    }

    fn cf_memo_tx(&self) -> Result<&ColumnFamily> {
        self.moonlight_db
            .cf_handle(CF_M_MEMO_TX)
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
            moonlight_tx_mappings,
        } = transformer::group_by_origins_filter_and_convert(
            block_events,
            block_height,
        );

        debug!(
            "Found {} moonlight transactions",
            moonlight_tx_mappings.len()
        );

        let address_inflow_mappings =
            util::check_duplicates(address_inflow_mappings);
        let address_outflow_mappings =
            util::check_duplicates(address_outflow_mappings);

        for mapping in address_inflow_mappings {
            let (pk, tx_hash) = mapping;
            self.update_inflow_address_tx(pk, tx_hash)?;
        }

        for mapping in address_outflow_mappings {
            let (pk, tx_hash) = mapping;
            self.update_outflow_address_tx(pk, tx_hash)?;
        }

        for MoonlightTxMapping(moonlight_tx, events) in moonlight_tx_mappings {
            self.insert_moonlight_events(moonlight_tx, events)?;
        }

        for memo_mapping in memo_mappings {
            let (memo, tx_hash) = memo_mapping;
            self.update_memo_tx(memo, tx_hash)?;
        }

        Ok(())
    }

    /// Insert or update an AccountPublicKey to TxHash mapping for inflows.
    fn update_inflow_address_tx(
        &self,
        pk: AccountPublicKey,
        moonlight_tx: MoonlightTx,
    ) -> Result<()> {
        let cf_inflow = self.cf_m_inflow_address_tx()?;
        let key = pk.to_bytes();

        self.append_moonlight_tx(cf_inflow, &key, moonlight_tx)
    }

    /// Insert or update an AccountPublicKey to TxHash mapping for outflows.
    fn update_outflow_address_tx(
        &self,
        pk: AccountPublicKey,
        moonlight_tx: MoonlightTx,
    ) -> Result<()> {
        let cf_outflow = self.cf_m_outflow_address_tx()?;
        let key = pk.to_bytes();

        self.append_moonlight_tx(cf_outflow, &key, moonlight_tx)
    }

    /// Insert or update a Memo to TxHash mapping.
    fn update_memo_tx(
        &self,
        memo: Vec<u8>,
        moonlight_tx: MoonlightTx,
    ) -> Result<()> {
        let cf_memo = self.cf_memo_tx()?;
        let key = memo;

        self.append_moonlight_tx(cf_memo, &key, moonlight_tx)
    }

    fn append_moonlight_tx(
        &self,
        cf: &ColumnFamily,
        key: &[u8],
        moonlight_tx: MoonlightTx,
    ) -> Result<()> {
        let txn = self.moonlight_db.transaction();

        let existing_tx_hashes = txn.get_cf(cf, key)?;

        if let Some(tx_hashes) = existing_tx_hashes {
            let mut moonlight_txs =
                serde_json::from_slice::<Vec<MoonlightTx>>(&tx_hashes)?;

            // Append the new TxHash to the existing tx hashes
            moonlight_txs.push(moonlight_tx);

            // Put the updated tx hashes back into the CF
            txn.put_cf(cf, key, serde_json::to_vec(&moonlight_txs)?)?;

            txn.commit()?;

            Ok(())
        } else {
            // Serialize the TxHash and put it into the CF
            txn.put_cf(cf, key, serde_json::to_vec(&vec![moonlight_tx])?)?;

            txn.commit()?;

            Ok(())
        }
    }

    /// Insert new moonlight event(s) for a MoonlightTx.
    fn insert_moonlight_events(
        &self,
        moonlight_tx: MoonlightTx,
        events: MoonlightTxEvents,
    ) -> Result<()> {
        let txn = self.moonlight_db.transaction();
        let cf = self.cf_txhash_moonlight_events()?;

        // Check if the MoonlightTx already exists in the database
        // If it does, return false, to not overwrite the existing events
        if txn.get_cf(cf, moonlight_tx.origin())?.is_some() {
            return Err(anyhow!(
                "MoonlightTx already exists. This should not happen"
            ));
        }

        // Serialize the events and put them into the database
        let v = serde_json::to_vec(&events)?;

        // We use the TxHash as the key
        txn.put_cf(cf, moonlight_tx.origin(), v)?;

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
    ) -> Result<Option<Vec<MoonlightGroup>>> {
        let MoonlightFlows { inflows, outflows } = self.moonlight_flows(pk)?;

        // Merge inflows and outflows
        let mut moonlight_txs = Vec::new();
        if let Some(inflows) = inflows {
            moonlight_txs.extend(inflows);
        }
        if let Some(outflows) = outflows {
            moonlight_txs.extend(outflows);
        }
        // Sort by block height to preserve the order
        // Note: We can merge the two vectors in a more efficient way because
        // they are already sorted
        moonlight_txs.sort_unstable_by_key(|tx| tx.block_height());

        if moonlight_txs.is_empty() {
            Ok(None)
        } else {
            self.moonlight_txs_events(moonlight_txs)
        }
    }

    /// Get a vector of Vec<MoonlightTxEvents> for a given memo.
    ///
    /// Clients are advised to check if the Vec is > 1, as memos are not unique.
    pub fn moonlight_txs_by_memo(
        &self,
        memo: Vec<u8>,
    ) -> Result<Option<Vec<MoonlightGroup>>> {
        if let Some(tx_hashes) = self.get_memo_txhashes(memo)? {
            self.moonlight_txs_events(tx_hashes)
        } else {
            Ok(None)
        }
    }

    /// Get a vector of Vec<MoonlightTxEvents> for a given list of MoonlightTx.
    pub fn moonlight_txs_events(
        &self,
        moonlight_tx: Vec<MoonlightTx>,
    ) -> Result<Option<Vec<MoonlightGroup>>> {
        let multi_get = self.multi_get_moonlight_events(&moonlight_tx);

        let mut moonlight_groups = Vec::with_capacity(multi_get.len());

        assert!(multi_get.len() == moonlight_tx.len());

        for (
            serialized_event,
            MoonlightTx {
                block_height,
                tx_hash,
            },
        ) in multi_get.iter().zip(moonlight_tx.iter())
        {
            if let Ok(Some(e)) = serialized_event {
                // Construct the MoonlightGroup from MoonlightTxEvents &
                // MoonlightTx
                let moonlight_tx_events =
                    serde_json::from_slice::<MoonlightTxEvents>(e)?;

                moonlight_groups.push(MoonlightGroup {
                    events: moonlight_tx_events.events(),
                    origin: *tx_hash,
                    block_height: *block_height,
                });
            } else {
                warn!("Serialized moonlight event not found");
                continue;
            }
        }

        if moonlight_groups.is_empty() {
            Ok(None)
        } else {
            Ok(Some(moonlight_groups))
        }
    }

    /// Get two vectors of MoonlightTx that relate to moonlight
    /// in- and outflows for a given AccountPublicKey.
    pub fn moonlight_flows(
        &self,
        address: AccountPublicKey,
    ) -> Result<MoonlightFlows> {
        Ok(MoonlightFlows {
            inflows: self.get_moonlight_inflow_tx(address)?,
            outflows: self.get_moonlight_outflow_tx(address)?,
        })
    }

    /// Get a vector of `MoonlightTx` that relate to moonlight
    /// transfers with the specified sender & receiver.
    ///
    /// `None` means any sender or receiver.
    /// If both sender and receiver are None, an error is returned.
    pub fn moonlight_transactions(
        &self,
        sender: Option<AccountPublicKey>,
        receiver: Option<AccountPublicKey>,
        from_block: Option<u64>,
        to_block: Option<u64>,
        max_count: Option<usize>,
        page_count: Option<usize>,
    ) -> Result<Option<Vec<MoonlightTx>>> {
        let max_count = max_count.unwrap_or(DEFAULT_MAX_COUNT);
        // None and Page 1 = 0, Page 2 = 1, Page 3 = 2, ...
        let page_count = page_count.map(|p| p - 1).unwrap_or(0);

        let moonlight_tx = match (sender, receiver) {
            (None, Some(receiver)) => self.get_moonlight_inflow_tx(receiver)?,

            (Some(sender), None) => self.get_moonlight_outflow_tx(sender)?,

            (Some(sender), Some(receiver)) if sender == receiver => {
                // If sender & receiver are the same, return only
                // outflows. (sending to self is stored as an outflow tx
                // which should be interpreted as a "Self" tx)
                self.get_moonlight_outflow_tx(sender)?
            }
            (Some(sender), Some(receiver)) =>
            // Intersection first and then limit
            {
                util::intersection(
                    self.get_moonlight_inflow_tx(receiver)?.unwrap_or_default(),
                    self.get_moonlight_outflow_tx(sender)?.unwrap_or_default(),
                )
            }

            _ => return Err(anyhow!("No sender or receiver provided")),
        };

        Ok(util::limit(
            moonlight_tx,
            from_block,
            to_block,
            max_count,
            page_count,
        ))
    }

    pub fn moonlight_groups(
        &self,
        sender: Option<AccountPublicKey>,
        receiver: Option<AccountPublicKey>,
        from_block: Option<u64>,
        to_block: Option<u64>,
        max_count: Option<usize>,
        page_count: Option<usize>,
    ) -> Result<Option<Vec<MoonlightGroup>>> {
        let moonlight_tx = self.moonlight_transactions(
            sender, receiver, from_block, to_block, max_count, page_count,
        )?;

        if let Some(moonlight_tx) = moonlight_tx {
            self.moonlight_txs_events(moonlight_tx)
        } else {
            Ok(None)
        }
    }

    fn get_moonlight_outflow_tx(
        &self,
        sender: AccountPublicKey,
    ) -> Result<Option<Vec<MoonlightTx>>> {
        // Note: We can likely only partially read (also with binary search)
        // the tx_hashes through wide_column & PinnableWideColumns
        if let Some(tx_hashes) = self
            .moonlight_db
            .get_cf(self.cf_m_outflow_address_tx()?, sender.to_bytes())?
        {
            Ok(Some(serde_json::from_slice::<Vec<MoonlightTx>>(
                &tx_hashes,
            )?))
        } else {
            Ok(None)
        }
    }

    fn get_moonlight_inflow_tx(
        &self,
        receiver: AccountPublicKey,
    ) -> Result<Option<Vec<MoonlightTx>>> {
        // Note: We can likely only partially read (also with binary search)
        // the tx_hashes through wide_column & PinnableWideColumns
        if let Some(tx_hashes) = self
            .moonlight_db
            .get_cf(self.cf_m_inflow_address_tx()?, receiver.to_bytes())?
        {
            Ok(Some(serde_json::from_slice::<Vec<MoonlightTx>>(
                &tx_hashes,
            )?))
        } else {
            Ok(None)
        }
    }

    /// Get a vector of TxHash that relate to moonlight
    /// in- or outflows for a given memo.
    pub fn get_memo_txhashes(
        &self,
        memo: Vec<u8>,
    ) -> Result<Option<Vec<MoonlightTx>>> {
        if let Some(moonlight_tx) =
            self.moonlight_db.get_cf(self.cf_memo_tx()?, memo)?
        {
            Ok(Some(serde_json::from_slice::<Vec<MoonlightTx>>(
                &moonlight_tx,
            )?))
        } else {
            Ok(None)
        }
    }

    /// Get data to construct MoonlightGroup for a given MoonlightTx.
    pub fn get_moonlight_events(
        &self,
        moonlight_tx: MoonlightTx,
    ) -> Result<Option<(MoonlightTx, Vec<u8>)>> {
        if let Some(events) = self
            .moonlight_db
            .get_cf(self.cf_txhash_moonlight_events()?, moonlight_tx.origin())?
        {
            Ok(Some((moonlight_tx, events)))
        } else {
            Ok(None)
        }
    }

    fn tx_multiget_keys<'a>(
        &'a self,
        moonlight_txs: &'a Vec<MoonlightTx>,
    ) -> Vec<&TxHash> {
        let mut keys = Vec::with_capacity(moonlight_txs.len());

        for moonlight_tx in moonlight_txs {
            keys.push(moonlight_tx.origin());
        }

        keys
    }

    /// Get multiple MoonlightGroups for a given list of MoonlightTx.
    fn multi_get_moonlight_events(
        &self,
        moonlight_txs: &Vec<MoonlightTx>,
    ) -> Vec<CoreResult<Option<DBPinnableSlice>, rocksdb::Error>> {
        let cf = match self.cf_txhash_moonlight_events() {
            Ok(cf) => cf,
            Err(e) => {
                error!("{}", e);
                return Vec::new();
            }
        };
        // ToDo: check what sorted_input for a difference makes
        self.moonlight_db.batched_multi_get_cf(
            cf,
            self.tx_multiget_keys(moonlight_txs),
            true,
        )
    }
}

mod util {
    use tracing::warn;

    use super::{AccountPublicKey, MoonlightTx, Serializable};

    pub(super) fn intersection(
        inflows: Vec<MoonlightTx>,
        outflows: Vec<MoonlightTx>,
    ) -> Option<Vec<MoonlightTx>> {
        let intersection = inflows
            .into_iter()
            .filter(|inflow_tx| {
                // Check if the MoonlightTx is in the outflows
                // 1. Binary search for the block height of inflow in outflow
                //    vector
                // 2. If the block height is found, check if the origin is the
                //    same
                // 3. If the origin is the same, we can yield the MoonlightTx
                outflows
                    .binary_search_by_key(&inflow_tx.block_height(), |tx| {
                        tx.block_height()
                    })
                    .ok()
                    .map_or(false, |idx| {
                        outflows[idx].origin() == inflow_tx.origin()
                    })
            })
            .collect::<Vec<MoonlightTx>>();

        if intersection.is_empty() {
            None
        } else {
            Some(intersection)
        }
    }

    pub(super) fn limit(
        moonlight_tx: Option<Vec<MoonlightTx>>,
        from_block: Option<u64>,
        to_block: Option<u64>,
        max_count: usize,
        page_count: usize,
    ) -> Option<Vec<MoonlightTx>> {
        let mut moonlight_tx = moonlight_tx.unwrap_or_default();

        if let Some(to_block) = to_block {
            // Remove all transactions that are above the to_block
            while moonlight_tx
                .last()
                .map_or(false, |tx| tx.block_height() > to_block)
            {
                moonlight_tx.pop();
            }
        }

        let lower_bound_idx: usize;
        if let Some(from_block) = from_block {
            // Find lower bound index (greater or equal from_block)
            lower_bound_idx = lower_bound(&moonlight_tx, from_block);
        } else {
            lower_bound_idx = 0;
        }

        // Skip to lower bound and take max_count, put the rest into cache
        let limited = moonlight_tx
            .into_iter()
            .skip(lower_bound_idx + (page_count * max_count))
            .take(max_count)
            .collect::<Vec<MoonlightTx>>();

        if limited.is_empty() {
            None
        } else {
            Some(limited)
        }
    }

    /// Find lower bound for MoonlightTx
    fn lower_bound(moonlight_tx: &Vec<MoonlightTx>, target: u64) -> usize {
        let mut left = 0;
        let mut right = moonlight_tx.len();

        while left < right {
            let mid = left + ((right - left) / 2);
            if moonlight_tx[mid].block_height() < target {
                left = mid + 1;
            } else {
                right = mid;
            }
        }

        left
    }

    /// Check and remove duplicates from a list of address mappings.
    pub(super) fn check_duplicates(
        address_mappings: Vec<(AccountPublicKey, MoonlightTx)>,
    ) -> Vec<(AccountPublicKey, MoonlightTx)> {
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
    use super::{
        AccountPublicKey, Archive, ContractTxEvent, MoonlightFlows,
        MoonlightTx, MoonlightTxEvents,
    };
    use execution_core::transfer::withdraw::WithdrawReceiver;
    use execution_core::transfer::{
        ConvertEvent, DepositEvent, MoonlightTransactionEvent, WithdrawEvent,
    };
    use execution_core::{ContractId, CONTRACT_ID_BYTES};
    use node_data::events::contract::{
        ContractEvent, WrappedContractId, TX_HASH_BYTES,
    };
    use rand::rngs::StdRng;
    use rand::{distributions::Alphanumeric, Rng, SeedableRng};
    use rand::{CryptoRng, RngCore};
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
            refund_info: Some(AccountPublicKey::default()),
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

    fn random_txhash_events<R: RngCore + CryptoRng>(
        amount: usize,
        mut rng: R,
    ) -> Vec<Vec<ContractTxEvent>> {
        let mut events = Vec::new();
        let pk = Some(AccountPublicKey::default());
        for _ in 1..=amount {
            let rand_hash = rng.gen::<[u8; 32]>();

            let event = vec![moonlight_event(rand_hash, pk, vec![0])];
            events.push(event);
        }
        events
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
            moonlight_tx_mappings,
        } = group_by_origins_filter_and_convert(block_events, 1);

        assert_eq!(address_outflow_mappings.len(), 3);
        assert_eq!(address_inflow_mappings.len(), 2);
        assert_eq!(memo_mappings.len(), 3);
        // 5 moonlight groups means 5 transactions containing moonlight related
        // events
        assert_eq!(moonlight_tx_mappings.len(), 5);
    }

    #[tokio::test]
    async fn test_tl_moonlight_and_fetch() {
        let path = test_dir();
        let archive = Archive::create_or_open(path).await;

        let pk = AccountPublicKey::default();
        assert!(archive.moonlight_txs_by_pk(pk).unwrap().is_none());

        let block_events = block_events();

        // Store block events in the archive
        archive.tl_moonlight(block_events, 1).unwrap();

        let MoonlightFlows { inflows, outflows } =
            archive.moonlight_flows(pk).unwrap();

        let fetched_moonlight_tx = inflows
            .unwrap()
            .into_iter()
            .chain(outflows.unwrap())
            .collect::<Vec<MoonlightTx>>();

        let fetched_events = archive.moonlight_txs_by_pk(pk).unwrap().unwrap();

        assert_eq!(fetched_moonlight_tx.len(), 5);

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
        archive.tl_moonlight(block_events, 1).unwrap();

        let MoonlightFlows { inflows, outflows } =
            archive.moonlight_flows(pk).unwrap();
        let fetched_moonlight_tx = inflows
            .unwrap()
            .into_iter()
            .chain(outflows.unwrap())
            .collect::<Vec<MoonlightTx>>();

        let (moonlight_tx, fetched_events_by_moonlight_tx) = archive
            .get_moonlight_events(fetched_moonlight_tx[0])
            .unwrap()
            .unwrap();
        let fetched_events_by_moonlight_tx =
            serde_json::from_slice::<MoonlightTxEvents>(
                &fetched_events_by_moonlight_tx,
            )
            .unwrap();

        assert_eq!(fetched_moonlight_tx[0].block_height(), 1);
        assert_eq!(fetched_moonlight_tx[0].origin(), &[1u8; TX_HASH_BYTES]);
        assert_eq!(fetched_moonlight_tx[0], moonlight_tx);

        let events = fetched_events_by_moonlight_tx.events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].topic, "convert");
    }

    #[tokio::test]
    async fn test_tl_memo_and_fetch_single() {
        let path = test_dir();
        let archive = Archive::create_or_open(path).await;

        let block_events = memo_txs();
        archive.tl_moonlight(block_events, 1).unwrap();

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

    #[tokio::test]
    async fn test_tl_moonlight_transfers() {
        let path = test_dir();
        let archive = Archive::create_or_open(path).await;
        let amount = 300;
        let mut rng = StdRng::seed_from_u64(1618u64);
        let block_events = random_txhash_events(amount, &mut rng);

        for (i, block_event) in block_events.into_iter().enumerate() {
            archive.tl_moonlight(block_event, (i + 1) as u64).unwrap();
        }

        // Transfers to yourself are outflows but not inflows
        assert!(archive
            .moonlight_transactions(
                None,
                Some(AccountPublicKey::default()),
                None,
                None,
                None,
                None
            )
            .unwrap()
            .is_none());

        // No limit
        let moonlight_txs = archive
            .moonlight_transactions(
                Some(AccountPublicKey::default()),
                None,
                None,
                None,
                None,
                None,
            )
            .unwrap()
            .unwrap();

        assert_eq!(moonlight_txs.len(), amount);

        // Reset the rng
        let mut rng = StdRng::seed_from_u64(1618u64);

        for (i, moonlight_tx) in moonlight_txs.iter().enumerate() {
            assert_eq!(moonlight_tx.origin(), &rng.gen::<[u8; 32]>());

            assert_eq!(moonlight_tx.block_height(), (i + 1) as u64);
        }

        // Limit from block height 100 to 150
        let moonlight_txs = archive
            .moonlight_transactions(
                Some(AccountPublicKey::default()),
                None,
                Some(100),
                Some(150),
                None,
                None,
            )
            .unwrap()
            .unwrap();

        assert_eq!(moonlight_txs.len(), 51); // [100, 150] = 51 elements

        for (i, moonlight_tx) in moonlight_txs.iter().enumerate() {
            assert_eq!(moonlight_tx.block_height(), (i + 100) as u64);
        }

        // Limit from block height 100 to 150 and both sender, receiver
        // specified Since all test data sends to own wallet, this
        // should be the same as moonlight_txs above
        let moonlight_txs_both = archive
            .moonlight_transactions(
                Some(AccountPublicKey::default()),
                Some(AccountPublicKey::default()),
                Some(100),
                Some(150),
                None,
                None,
            )
            .unwrap()
            .unwrap();

        assert_eq!(moonlight_txs_both.len(), 51);
        assert_eq!(moonlight_txs, moonlight_txs_both);

        // Limit from block height 100 to 150 but max_count is 5
        let moonlight_txs = archive
            .moonlight_transactions(
                Some(AccountPublicKey::default()),
                None,
                Some(100),
                Some(150),
                Some(5),
                None,
            )
            .unwrap()
            .unwrap();

        assert_eq!(moonlight_txs.len(), 5);

        for (i, moonlight_tx) in moonlight_txs.iter().enumerate() {
            assert_eq!(moonlight_tx.block_height(), (i + 100) as u64);
        }

        // Limit from block height 100 to 150 but max_count is 1
        // and page_count is used
        for p in 1..=5 {
            let moonlight_txs = archive
                .moonlight_transactions(
                    Some(AccountPublicKey::default()),
                    None,
                    Some(100),
                    Some(150),
                    Some(1),
                    Some(p),
                )
                .unwrap()
                .unwrap();

            assert_eq!(moonlight_txs.len(), 1);

            let p = p - 1;
            for (i, moonlight_tx) in moonlight_txs.iter().enumerate() {
                assert_eq!(moonlight_tx.block_height(), (i + (100 + p)) as u64);
            }
        }
    }
}
