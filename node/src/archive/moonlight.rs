// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use core::result::Result as CoreResult;
use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use dusk_bytes::Serializable;
use dusk_core::signatures::bls::PublicKey as AccountPublicKey;
use node_data::events::contract::{ContractEvent, OriginHash};
use rocksdb::{
    BlockBasedOptions, ColumnFamily, ColumnFamilyDescriptor, DBPinnableSlice,
    LogLevel, OptimisticTransactionDB, Options,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, warn};

use crate::archive::transformer::{
    self, EventIdentifier, MoonlightTransferEvents, MoonlightTransferMapping,
};
use crate::archive::{Archive, ArchiveOptions};

/// Subfolder containing the moonlight database.
const MOONLIGHT_DB_FOLDER_NAME: &str = "moonlight.db";

/// Default max count for moonlight transfers returned.
const DEFAULT_MAX_COUNT: usize = 1000;

// Column family names.

/// Moonlight TxHash to MoonlightTxEvents mapping
const CF_MTXHASH_MEVENTS: &str = "cf_mtxhash_mevents";
/// AccountPublicKey to Inflow MoonlightTx mapping
const CF_M_INFLOW_ADDRESS_TX: &str = "cf_m_inflow_address_tx";
/// AccountPublicKey to Outflow MoonlightTx mapping
const CF_M_OUTFLOW_ADDRESS_TX: &str = "cf_m_outflow_address_tx";
/// Memo to MoonlightTx mapping (in- & outlfows)
const CF_M_MEMO_TX: &str = "cf_m_memo_tx";

/// Order of the transfers.
///
/// Ascending means the oldest transfer is first and the newest is last.
/// Descending means the newest transfer is first and the oldest is last.
///
/// New & old is defined by the block height. The higher the block height, the
/// newer the transfer.
pub enum Order {
    /// Ascending from oldest to newest.
    Ascending,
    /// Descending from newest to oldest.
    Descending,
}

/// Group of events belonging to a single **transaction** and
/// additional metadata.
///
/// One transaction can contain multiple events and multiple transfers
/// of assets. The underlying Vec<ContractEvent> contains at least one event
/// that relates to a moonlight in- or outflow.
///
/// This can be a "moonlight" event or
/// a "withdraw", "contract_to_account", "mint", or "convert" event
#[serde_with::serde_as]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoonlightGroup {
    events: Vec<ContractEvent>,
    #[serde_as(as = "serde_with::hex::Hex")]
    origin: OriginHash,
    block_height: u64,
}

impl MoonlightGroup {
    /// Returns the events of the MoonlightGroup.
    pub fn events(&self) -> &[ContractEvent] {
        &self.events
    }

    /// Returns the origin of the MoonlightGroup/Events.
    pub fn origin(&self) -> &OriginHash {
        &self.origin
    }

    /// Returns the block height of the MoonlightGroup/Events.
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

    /// Transform & Load moonlight related events into the moonlight database.
    ///
    ///
    /// # Arguments
    ///
    /// * `grouped_events` - List of ContractTxEvent grouped by TxIdentifier
    ///   from a finalized block.
    ///
    /// # Returns
    ///
    /// * `active_accounts` - List of active accounts from the transactions
    pub(super) fn tl_moonlight(
        &self,
        grouped_events: BTreeMap<EventIdentifier, Vec<ContractEvent>>,
    ) -> Result<HashSet<String>> {
        debug!("Loading transaction events into the moonlight db");

        let transformer::TransormerResult {
            address_outflow_mappings,
            address_inflow_mappings,
            memo_mappings,
            moonlight_tx_mappings,
        } = transformer::filter_and_convert(grouped_events);

        debug!("Found {} moonlight transfers", moonlight_tx_mappings.len());

        let address_inflow_mappings =
            util::check_duplicates(address_inflow_mappings);
        let address_outflow_mappings =
            util::check_duplicates(address_outflow_mappings);

        let mut active_accounts = HashSet::with_capacity(
            address_inflow_mappings.len() + address_outflow_mappings.len(),
        );

        for mapping in address_inflow_mappings {
            let (pk, tx_hash) = mapping;
            active_accounts.insert(bs58::encode(pk.to_bytes()).into_string());
            self.update_inflow_address_tx(pk, tx_hash)?;
        }

        for mapping in address_outflow_mappings {
            let (pk, tx_hash) = mapping;
            active_accounts.insert(bs58::encode(pk.to_bytes()).into_string());
            self.update_outflow_address_tx(pk, tx_hash)?;
        }

        for MoonlightTransferMapping(moonlight_tx, events) in
            moonlight_tx_mappings
        {
            self.put_moonlight_events(moonlight_tx, events)?;
        }

        for memo_mapping in memo_mappings {
            let (memo, tx_hash) = memo_mapping;
            self.update_memo_tx(memo, tx_hash)?;
        }

        Ok(active_accounts)
    }

    /// Insert or update an AccountPublicKey to MoonlightTx mapping for inflows.
    fn update_inflow_address_tx(
        &self,
        pk: AccountPublicKey,
        moonlight_tx: EventIdentifier,
    ) -> Result<()> {
        let key = pk.to_bytes();
        self.append_moonlight_tx(
            self.cf_m_inflow_address_tx()?,
            &key,
            moonlight_tx,
        )
    }

    /// Insert or update an AccountPublicKey to MoonlightTx mapping for
    /// outflows.
    fn update_outflow_address_tx(
        &self,
        pk: AccountPublicKey,
        moonlight_tx: EventIdentifier,
    ) -> Result<()> {
        let key = pk.to_bytes();
        self.append_moonlight_tx(
            self.cf_m_outflow_address_tx()?,
            &key,
            moonlight_tx,
        )
    }

    /// Insert or update a Memo to MoonlightTx mapping.
    fn update_memo_tx(
        &self,
        memo: Vec<u8>,
        moonlight_tx: EventIdentifier,
    ) -> Result<()> {
        self.append_moonlight_tx(self.cf_memo_tx()?, &memo, moonlight_tx)
    }

    /// Get the full moonlight transfer history of a given AccountPublicKey.
    ///
    /// Returns all finalized moonlight events affecting the balance of the
    /// given public key
    pub fn full_moonlight_history(
        &self,
        pk: AccountPublicKey,
        ord: Option<Order>,
        from_block: Option<u64>,
        to_block: Option<u64>,
    ) -> Result<Option<Vec<MoonlightGroup>>> {
        let order = ord.unwrap_or(Order::Ascending);

        let inflows = self.fetch_moonlight_history(
            Some(pk),
            None,
            from_block,
            to_block,
            None,
            None,
        )?;
        let outflows = self.fetch_moonlight_history(
            None,
            Some(pk),
            from_block,
            to_block,
            None,
            None,
        )?;

        // Merge inflows and outflows
        let mut moonlight_groups = Vec::new();
        if let Some(inflows) = inflows {
            moonlight_groups.extend(inflows);
        }
        if let Some(outflows) = outflows {
            moonlight_groups.extend(outflows);
        }
        // Sort by block height to preserve the order
        // Note: We can do a more efficient merge of the two vectors in the
        // future because they are already sorted.
        moonlight_groups
            .sort_unstable_by_key(|tx| (tx.block_height(), *tx.origin()));
        // Remove all duplicates (can be, if tx to self were sent)
        moonlight_groups.dedup();

        if let Order::Descending = order {
            moonlight_groups.reverse();
        }

        if moonlight_groups.is_empty() {
            Ok(None)
        } else {
            Ok(Some(moonlight_groups))
        }
    }

    /// Get a vector of MoonlightGroup for a given memo.
    pub fn moonlight_txs_by_memo(
        &self,
        memo: Vec<u8>,
    ) -> Result<Option<Vec<MoonlightGroup>>> {
        if let Some(tx_hashes) = self.get_memo_txhashes(memo)? {
            self.moonlight_groups(tx_hashes)
        } else {
            Ok(None)
        }
    }

    /// Get a vector of MoonlightGroup for a given vector of MoonlightTx.
    fn moonlight_groups(
        &self,
        moonlight_tx: Vec<EventIdentifier>,
    ) -> Result<Option<Vec<MoonlightGroup>>> {
        let multi_get = self.multi_get_moonlight_events(&moonlight_tx);

        let mut moonlight_groups = Vec::with_capacity(multi_get.len());

        debug!(
            "Found {} MoonlightTxEvents for {} MoonlightTx",
            multi_get.len(),
            moonlight_tx.len()
        );

        for (
            serialized_event,
            EventIdentifier {
                block_height,
                tx_hash,
            },
        ) in multi_get.iter().zip(moonlight_tx.iter())
        {
            if let Ok(Some(e)) = serialized_event {
                // Construct the MoonlightGroup from MoonlightTxEvents &
                // MoonlightTx
                let moonlight_tx_events =
                    serde_json::from_slice::<MoonlightTransferEvents>(e)?;

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

    /// Get a vector of `EventIdentifier` that relate to moonlight
    /// transfers with the specified sender & receiver.
    ///
    ///
    /// # Arguments
    ///
    /// * `sender` - The sender of the transfer.
    /// * `receiver` - The receiver of the transfer.
    /// * `from_block` - The block height from which to start fetching
    /// * `to_block` - The block height until which to fetch
    /// * `max_count` - The maximum number of transactions to fetch
    /// * `page_count` - The page count for the transactions (Pagination with
    ///   max_count per page)
    ///
    ///
    /// `None` means any sender or receiver.
    /// If both sender and receiver are None, an error is returned.
    /// If both sender and receiver are Some, the intersection of transactions
    /// is returned.
    pub fn fetch_moonlight_event_ident(
        &self,
        sender: Option<AccountPublicKey>,
        receiver: Option<AccountPublicKey>,
        from_block: Option<u64>,
        to_block: Option<u64>,
        max_count: Option<usize>,
        page_count: Option<usize>,
    ) -> Result<Option<Vec<EventIdentifier>>> {
        let max_count = max_count.unwrap_or(DEFAULT_MAX_COUNT);
        // None and Page 1 = 0, Page 2 = 1, Page 3 = 2, ...
        let page_count = page_count.map(|p| p - 1).unwrap_or(0);

        let moonlight_tx = match (sender, receiver) {
            (None, Some(receiver)) => self.get_moonlight_inflow_tx(receiver)?,
            (Some(sender), None) => self.get_moonlight_outflow_tx(sender)?,
            (Some(sender), Some(receiver)) => util::intersection(
                self.get_moonlight_inflow_tx(receiver)?.unwrap_or_default(),
                self.get_moonlight_outflow_tx(sender)?.unwrap_or_default(),
            ),

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

    /// Get a vector of `MoonlightGroup` that relate to moonlight
    /// transfers with the specified sender & receiver.
    ///
    ///
    /// # Arguments
    ///
    /// * `sender` - The sender of the transfer.
    /// * `receiver` - The receiver of the transfer.
    /// * `from_block` - The block height from which to start fetching
    /// * `to_block` - The block height until which to fetch
    /// * `max_count` - The maximum number of transactions to fetch
    /// * `page_count` - The page count for the transactions (Pagination with
    ///   max_count per page)
    ///
    ///
    /// `None` means any sender or receiver.
    /// If both sender and receiver are None, an error is returned.
    /// If both sender and receiver are Some, the intersection of transactions
    /// is returned.
    pub fn fetch_moonlight_history(
        &self,
        sender: Option<AccountPublicKey>,
        receiver: Option<AccountPublicKey>,
        from_block: Option<u64>,
        to_block: Option<u64>,
        max_count: Option<usize>,
        page_count: Option<usize>,
    ) -> Result<Option<Vec<MoonlightGroup>>> {
        let moonlight_tx = self.fetch_moonlight_event_ident(
            sender, receiver, from_block, to_block, max_count, page_count,
        )?;

        if let Some(moonlight_tx) = moonlight_tx {
            self.moonlight_groups(moonlight_tx)
        } else {
            Ok(None)
        }
    }
}

/// Methods that interact directly with rocksdb.
impl Archive {
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

    /// Append a new moonlight transaction to the existing list of
    /// moonlight transactions for a given key.
    ///
    /// If the key does not exist, it will be created.
    fn append_moonlight_tx(
        &self,
        cf: &ColumnFamily,
        key: &[u8],
        moonlight_tx: EventIdentifier,
    ) -> Result<()> {
        let txn = self.moonlight_db.transaction();

        let existing_tx_hashes = txn.get_cf(cf, key)?;

        if let Some(tx_hashes) = existing_tx_hashes {
            let mut moonlight_txs =
                serde_json::from_slice::<Vec<EventIdentifier>>(&tx_hashes)?;

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
    fn put_moonlight_events(
        &self,
        moonlight_tx: EventIdentifier,
        events: MoonlightTransferEvents,
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

    fn get_moonlight_outflow_tx(
        &self,
        sender: AccountPublicKey,
    ) -> Result<Option<Vec<EventIdentifier>>> {
        // Note: We can likely only partially read (also with binary search)
        // the tx_hashes through wide_column & PinnableWideColumns
        if let Some(tx_hashes) = self
            .moonlight_db
            .get_cf(self.cf_m_outflow_address_tx()?, sender.to_bytes())?
        {
            Ok(Some(serde_json::from_slice::<Vec<EventIdentifier>>(
                &tx_hashes,
            )?))
        } else {
            Ok(None)
        }
    }

    fn get_moonlight_inflow_tx(
        &self,
        receiver: AccountPublicKey,
    ) -> Result<Option<Vec<EventIdentifier>>> {
        // Note: We can likely only partially read (also with binary search)
        // the tx_hashes through wide_column & PinnableWideColumns
        if let Some(tx_hashes) = self
            .moonlight_db
            .get_cf(self.cf_m_inflow_address_tx()?, receiver.to_bytes())?
        {
            Ok(Some(serde_json::from_slice::<Vec<EventIdentifier>>(
                &tx_hashes,
            )?))
        } else {
            Ok(None)
        }
    }

    /// Get a vector of MoonlightTx that relate to moonlight
    /// in- or outflows for a given memo.
    fn get_memo_txhashes(
        &self,
        memo: Vec<u8>,
    ) -> Result<Option<Vec<EventIdentifier>>> {
        if let Some(moonlight_tx) =
            self.moonlight_db.get_cf(self.cf_memo_tx()?, memo)?
        {
            Ok(Some(serde_json::from_slice::<Vec<EventIdentifier>>(
                &moonlight_tx,
            )?))
        } else {
            Ok(None)
        }
    }

    /// Get data to construct MoonlightGroup for a given MoonlightTx.
    pub fn get_moonlight_events(
        &self,
        moonlight_tx: EventIdentifier,
    ) -> Result<Option<(EventIdentifier, Vec<u8>)>> {
        if let Some(events) = self
            .moonlight_db
            .get_cf(self.cf_txhash_moonlight_events()?, moonlight_tx.origin())?
        {
            Ok(Some((moonlight_tx, events)))
        } else {
            Ok(None)
        }
    }

    /// Get multiple MoonlightTxEvents for a given list of MoonlightTx.
    fn multi_get_moonlight_events(
        &self,
        moonlight_txs: &[EventIdentifier],
    ) -> Vec<CoreResult<Option<DBPinnableSlice>, rocksdb::Error>> {
        let cf = match self.cf_txhash_moonlight_events() {
            Ok(cf) => cf,
            Err(e) => {
                error!("{}", e);
                return Vec::new();
            }
        };

        let keys: Vec<&OriginHash> =
            moonlight_txs.iter().map(|tx| tx.origin()).collect();

        // sorted_input - If true, it means the input keys are already sorted by
        // key order, so the MultiGet() API doesn't have to sort them again.
        // https://github.com/facebook/rocksdb/blob/632746bb5b8d9d817b0075b295e1a085e1e543a4/include/rocksdb/c.h#L573
        self.moonlight_db.batched_multi_get_cf(cf, keys, true)
    }
}

mod util {
    use tracing::warn;

    use super::{AccountPublicKey, EventIdentifier, Serializable};

    /// Return the intersection of two vectors of MoonlightTx.
    pub(super) fn intersection(
        inflows: Vec<EventIdentifier>,
        outflows: Vec<EventIdentifier>,
    ) -> Option<Vec<EventIdentifier>> {
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
            .collect::<Vec<EventIdentifier>>();

        if intersection.is_empty() {
            None
        } else {
            Some(intersection)
        }
    }

    /// Limit the number of MoonlightTx returned based on the passed arguments.
    pub(super) fn limit(
        moonlight_tx: Option<Vec<EventIdentifier>>,
        from_block: Option<u64>,
        to_block: Option<u64>,
        max_count: usize,
        page_count: usize,
    ) -> Option<Vec<EventIdentifier>> {
        if let Some(mut moonlight_tx) = moonlight_tx {
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
                // Find lower bound index (for value greater or equal
                // from_block)
                lower_bound_idx = lower_bound(&moonlight_tx, from_block);
            } else {
                lower_bound_idx = 0;
            }

            // Skip to lower bound and take max_count * page_count
            let limited = moonlight_tx
                .into_iter()
                .skip(lower_bound_idx + (page_count * max_count))
                .take(max_count)
                .collect::<Vec<EventIdentifier>>();

            if limited.is_empty() {
                None
            } else {
                Some(limited)
            }
        } else {
            None
        }
    }

    /// Find lower bound for MoonlightTx.
    fn lower_bound(moonlight_tx: &Vec<EventIdentifier>, target: u64) -> usize {
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
        address_mappings: Vec<(AccountPublicKey, EventIdentifier)>,
    ) -> Vec<(AccountPublicKey, EventIdentifier)> {
        let len_before = address_mappings.len();
        let mut seen = std::collections::HashSet::new();
        let mut deduped = Vec::new();

        for (pk, txh) in address_mappings {
            if seen.insert((pk.to_bytes(), txh)) {
                deduped.push((pk, txh));
            }
        }

        if len_before != deduped.len() {
            warn!("Found duplicates in address mappings for transactions. Duplicates have been removed. This is a bug.");
        }

        deduped
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::path::PathBuf;

    use dusk_core::abi::{ContractId, CONTRACT_ID_BYTES};
    use dusk_core::signatures::bls::SecretKey;
    use dusk_core::transfer::withdraw::WithdrawReceiver;
    use dusk_core::transfer::{
        ConvertEvent, DepositEvent, MoonlightTransactionEvent, WithdrawEvent,
    };
    use node_data::events::contract::{
        ContractEvent, ContractTxEvent, ORIGIN_HASH_BYTES,
    };
    use rand::distributions::Alphanumeric;
    use rand::rngs::StdRng;
    use rand::{CryptoRng, Rng, RngCore, SeedableRng};

    use super::transformer::{self, filter_and_convert, TransormerResult};
    use super::{
        AccountPublicKey, Archive, EventIdentifier, MoonlightTransferEvents,
    };

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
                target: ContractId::from_bytes([0; CONTRACT_ID_BYTES]),
                topic: topic.to_owned(),
                data: vec![1, 6, 1, 8],
            },
            origin: [0; 32],
        }
    }

    fn phoenix_event() -> ContractTxEvent {
        let fake_phoenix_tx_event_data = vec![0, 0, 0, 0, 0];

        ContractTxEvent {
            event: ContractEvent {
                target: dusk_core::transfer::TRANSFER_CONTRACT,

                topic: "phoenix".to_string(),
                data: rkyv::to_bytes::<_, 256>(&fake_phoenix_tx_event_data)
                    .unwrap()
                    .to_vec(),
            },
            origin: [0; 32],
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
                target: dusk_core::transfer::TRANSFER_CONTRACT,

                topic: "convert".to_string(),
                data: rkyv::to_bytes::<_, 256>(&convert_event)
                    .unwrap()
                    .to_vec(),
            },
            origin: [1; 32],
        }
    }

    fn moonlight_event(
        origin: [u8; 32],
        sender: AccountPublicKey,
        receiver: Option<AccountPublicKey>,
        memo: Vec<u8>,
        refund_info: Option<(AccountPublicKey, u64)>,
    ) -> ContractTxEvent {
        let moonlight_tx_event = MoonlightTransactionEvent {
            sender,
            receiver,
            value: 500,
            memo,
            gas_spent: 500,
            refund_info,
        };

        ContractTxEvent {
            event: ContractEvent {
                target: dusk_core::transfer::TRANSFER_CONTRACT,

                topic: "moonlight".to_string(),
                data: rkyv::to_bytes::<_, 256>(&moonlight_tx_event)
                    .unwrap()
                    .to_vec(),
            },
            origin: origin,
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
                target: dusk_core::transfer::TRANSFER_CONTRACT,
                topic: "withdraw".to_string(),
                data: rkyv::to_bytes::<_, 256>(&withdraw_event)
                    .unwrap()
                    .to_vec(),
            },
            origin: [3; 32],
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
                target: dusk_core::transfer::TRANSFER_CONTRACT,

                topic: "deposit".to_string(),
                data: rkyv::to_bytes::<_, 256>(&deposit_event)
                    .unwrap()
                    .to_vec(),
            },
            origin: origin,
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
                target: dusk_core::transfer::TRANSFER_CONTRACT,
                topic: "deposit".to_string(),
                data: rkyv::to_bytes::<_, 256>(&deposit_event)
                    .unwrap()
                    .to_vec(),
            },
            origin: [5; 32],
        }
    }

    fn block_events() -> Vec<ContractTxEvent> {
        let pk: AccountPublicKey = AccountPublicKey::default();

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
            moonlight_event([2; 32], pk, Some(pk), vec![0, 1, 1, 0], None),
            moonlight_event([9; 32], pk, Some(pk), vec![0, 1, 1, 0], None),
            moonlight_event(
                [6; 32],
                pk,
                Some(pk),
                vec![],
                Some((
                    AccountPublicKey::from(&SecretKey::random(
                        &mut StdRng::seed_from_u64(1),
                    )),
                    100,
                )),
            ),
            withdraw_event_moonlight(),
            // belongs together with deposit_event_phoenix
            moonlight_event([5; 32], pk, None, vec![0, 1, 1, 0], None),
            deposit_event_phoenix(),
        ]
    }

    fn random_events<R: RngCore + CryptoRng>(
        amount: usize,
        mut r_pk: Option<R>,
        mut r_tx_hash: R,
    ) -> (
        Vec<AccountPublicKey>,
        Vec<AccountPublicKey>,
        Vec<Vec<ContractTxEvent>>,
    ) {
        let mut events = Vec::new();
        let mut sender = AccountPublicKey::default();
        let mut receiver = sender;
        let mut senders = Vec::new();
        let mut receivers = Vec::new();
        for _ in 1..=amount {
            {
                if let Some(ref mut r_pk) = r_pk {
                    sender = AccountPublicKey::from(&SecretKey::random(r_pk));
                    receiver = AccountPublicKey::from(&SecretKey::random(r_pk));
                    senders.push(sender);
                    receivers.push(receiver);
                };
            };

            let rand_hash = r_tx_hash.gen::<[u8; 32]>();

            let event = vec![moonlight_event(
                rand_hash,
                sender,
                Some(receiver),
                vec![0],
                None,
            )];
            events.push(event);
        }
        (senders, receivers, events)
    }

    fn memo_txs() -> Vec<ContractTxEvent> {
        let pk = AccountPublicKey::default();
        vec![
            moonlight_event([0; 32], pk, None, vec![0, 1, 1, 0], None),
            moonlight_event([1; 32], pk, None, vec![0, 1, 1, 0], None),
            moonlight_event([2; 32], pk, None, vec![0, 1, 1, 0], None),
            moonlight_event([3; 32], pk, None, vec![0, 1, 1, 0], None),
            moonlight_event([4; 32], pk, None, vec![1, 1, 1, 1], None),
        ]
    }

    #[tokio::test]
    async fn test_event_transformer() {
        let block_events = block_events();
        let event_groups = transformer::group_by_origins(block_events, 1);
        let TransormerResult {
            address_outflow_mappings,
            address_inflow_mappings,
            memo_mappings,
            moonlight_tx_mappings,
        } = filter_and_convert(event_groups);

        assert_eq!(address_outflow_mappings.len(), 4);
        assert_eq!(address_inflow_mappings.len(), 6);
        // combine both, 4+6 = 10 moonlight tx
        let mut address_flow_mappings = address_outflow_mappings;
        address_flow_mappings.extend(address_inflow_mappings);
        address_flow_mappings.sort_by_key(|(_, mtx)| mtx.origin().clone());

        address_flow_mappings.dedup();

        // Now it should be 6, 3 less because 3 tx were the sent to
        // self and are now duplicates in the inflow & outflow list
        assert_eq!(address_flow_mappings.len(), 7);

        println!("{:?}", memo_mappings);
        assert_eq!(memo_mappings.len(), 3);

        // 6 moonlight groups means 6 tx containing moonlight related
        // events
        assert_eq!(moonlight_tx_mappings.len(), 6);
    }

    #[tokio::test]
    async fn test_tl_moonlight_and_fetch() {
        let path = test_dir();
        let archive = Archive::create_or_open(path).await;

        let pk = AccountPublicKey::default();
        assert!(archive
            .full_moonlight_history(pk, None, None, None, None)
            .unwrap()
            .is_none());

        let block_events = block_events();

        let event_groups = transformer::group_by_origins(block_events, 1);

        // Store block events in the archive
        archive.tl_moonlight(event_groups).unwrap();

        let inflows = archive.get_moonlight_inflow_tx(pk).unwrap();
        let outflows = archive.get_moonlight_outflow_tx(pk).unwrap();

        // Unwrap and combine inflows and outflows
        let mut fetched_moonlight_tx = inflows
            .unwrap()
            .into_iter()
            .chain(outflows.unwrap())
            .collect::<Vec<EventIdentifier>>();

        assert_eq!(fetched_moonlight_tx.len(), 9);

        fetched_moonlight_tx.sort_by_key(|mtx| mtx.origin().clone());
        fetched_moonlight_tx.dedup();
        assert_eq!(fetched_moonlight_tx.len(), 6);

        let fetched_events = archive
            .full_moonlight_history(pk, None, None, None, None)
            .unwrap()
            .unwrap();
        assert_eq!(fetched_events.len(), 6);

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
                [6, 6, ..] => {
                    assert_eq!(moonlight_events.events().len(), 1);
                    assert_eq!(moonlight_events.events()[0].topic, "moonlight");

                    let data = rkyv::from_bytes::<MoonlightTransactionEvent>(
                        &moonlight_events.events()[0].data,
                    )
                    .unwrap();

                    assert_eq!(
                        data.refund_info.unwrap(),
                        (
                            AccountPublicKey::from(&SecretKey::random(
                                &mut StdRng::seed_from_u64(1),
                            )),
                            100
                        )
                    );
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
        assert!(archive
            .full_moonlight_history(pk, None, None, None, None)
            .unwrap()
            .is_none());

        let block_events = block_events();
        let event_groups = transformer::group_by_origins(block_events, 1);
        archive.tl_moonlight(event_groups).unwrap();

        let inflows = archive.get_moonlight_inflow_tx(pk).unwrap();
        let outflows = archive.get_moonlight_outflow_tx(pk).unwrap();
        let fetched_moonlight_tx = inflows
            .unwrap()
            .into_iter()
            .chain(outflows.unwrap())
            .collect::<Vec<EventIdentifier>>();

        let (moonlight_tx, fetched_events_by_moonlight_tx) = archive
            .get_moonlight_events(fetched_moonlight_tx[0])
            .unwrap()
            .unwrap();
        let fetched_events_by_moonlight_tx =
            serde_json::from_slice::<MoonlightTransferEvents>(
                &fetched_events_by_moonlight_tx,
            )
            .unwrap();

        assert_eq!(fetched_moonlight_tx[0].block_height(), 1);
        assert_eq!(fetched_moonlight_tx[0].origin(), &[1u8; ORIGIN_HASH_BYTES]);
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
        let event_groups = transformer::group_by_origins(block_events, 1);
        archive.tl_moonlight(event_groups).unwrap();

        let fetched_tx1 = archive
            .moonlight_txs_by_memo(vec![1, 1, 1, 1])
            .unwrap()
            .unwrap();
        assert_eq!(fetched_tx1.len(), 1);
        assert_eq!(fetched_tx1[0].origin(), &[4; 32]);
        fetched_tx1[0].events().iter().for_each(|e| {
            assert_eq!(e.topic, "moonlight");
            assert_eq!(e.target, dusk_core::transfer::TRANSFER_CONTRACT);

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
            assert_eq!(fetched_tx.origin(), &[i as u8; ORIGIN_HASH_BYTES]);
            fetched_tx.events().iter().for_each(|e| {
                assert_eq!(e.topic, "moonlight");
                assert_eq!(e.target, dusk_core::transfer::TRANSFER_CONTRACT);

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
    async fn test_tl_moonlight_transfers_to_self() {
        let path = test_dir();
        let archive = Archive::create_or_open(path).await;
        let amount = 300;
        let mut rng = StdRng::seed_from_u64(1618u64);
        let (_, _, block_events) = random_events(amount, None, &mut rng);

        for (i, block_event) in block_events.into_iter().enumerate() {
            let grouped_events =
                transformer::group_by_origins(block_event, (i + 1) as u64);
            archive.tl_moonlight(grouped_events).unwrap();
        }

        // Receiver only
        let moonlight_txs = archive
            .fetch_moonlight_event_ident(
                None,
                Some(AccountPublicKey::default()),
                None,
                None,
                None,
                None,
            )
            .unwrap()
            .unwrap();

        assert_eq!(moonlight_txs.len(), amount);

        // Sender only
        let moonlight_txs = archive
            .fetch_moonlight_event_ident(
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
            .fetch_moonlight_event_ident(
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
            .fetch_moonlight_event_ident(
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
            .fetch_moonlight_event_ident(
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
                .fetch_moonlight_event_ident(
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

    #[tokio::test]
    async fn test_tl_moonlight_transfers_rnd() {
        let path = test_dir();
        let archive = Archive::create_or_open(path).await;
        let amount = 200;
        let mut r_txhash = StdRng::seed_from_u64(1618u64);
        let mut r_address = StdRng::seed_from_u64(1618u64);
        let (senders, receivers, block_events) =
            random_events(amount, Some(&mut r_address), &mut r_txhash);

        for (i, block_event) in block_events.into_iter().enumerate() {
            let event_groups =
                transformer::group_by_origins(block_event, (i + 1) as u64);

            archive.tl_moonlight(event_groups).unwrap();
        }

        for i in 0..amount {
            // sender only
            let s_moonlight_txs = archive
                .fetch_moonlight_event_ident(
                    Some(senders[i]),
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .unwrap()
                .unwrap();

            assert_eq!(s_moonlight_txs.len(), 1);

            // receiver only
            let r_moonlight_txs = archive
                .fetch_moonlight_event_ident(
                    None,
                    Some(receivers[i]),
                    None,
                    None,
                    None,
                    None,
                )
                .unwrap()
                .unwrap();

            assert_eq!(r_moonlight_txs.len(), 1);

            // both sender and receiver
            let s_r_moonlight_txs = archive
                .fetch_moonlight_event_ident(
                    Some(senders[i]),
                    Some(receivers[i]),
                    None,
                    None,
                    None,
                    None,
                )
                .unwrap()
                .unwrap();

            assert_eq!(s_r_moonlight_txs.len(), 1);

            assert_eq!(s_moonlight_txs, s_r_moonlight_txs);
            assert_eq!(r_moonlight_txs, s_r_moonlight_txs);
        }

        // Limit from block height 100 to 150
        let num = 100;
        let moonlight_txs = archive
            .fetch_moonlight_event_ident(
                Some(senders[num]),
                None,
                Some((num + 1) as u64),
                Some((num + 51) as u64),
                None,
                None,
            )
            .unwrap()
            .unwrap();

        assert_eq!(moonlight_txs.len(), 1);
        assert_eq!(moonlight_txs[0].block_height(), (num + 1) as u64);

        // Limit from block height 100 to 150 and both sender, receiver
        // specified
        let moonlight_txs_both = archive
            .fetch_moonlight_event_ident(
                Some(senders[num]),
                Some(receivers[num]),
                Some((num + 1) as u64),
                Some((num + 51) as u64),
                None,
                None,
            )
            .unwrap()
            .unwrap();

        assert_eq!(moonlight_txs_both.len(), 1);
        assert_eq!(moonlight_txs, moonlight_txs_both);

        // Limit from block height 100 to 150 but max_count is 0
        assert!(archive
            .fetch_moonlight_event_ident(
                Some(senders[num]),
                None,
                Some((num + 1) as u64),
                Some((num + 51) as u64),
                Some(0),
                None,
            )
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn test_tl_moonlight_fetch_ord() {
        let path = test_dir();
        let archive = Archive::create_or_open(path).await;

        let pk = AccountPublicKey::default();
        assert!(archive
            .full_moonlight_history(pk, None, None, None, None)
            .unwrap()
            .is_none());

        let block_events = block_events();

        let event_groups = transformer::group_by_origins(block_events, 1);

        // Store block events in the archive
        archive.tl_moonlight(event_groups).unwrap();

        let inflows = archive.get_moonlight_inflow_tx(pk).unwrap();
        let outflows = archive.get_moonlight_outflow_tx(pk).unwrap();

        // Unwrap and combine inflows and outflows
        let mut fetched_moonlight_tx = inflows
            .unwrap()
            .into_iter()
            .chain(outflows.unwrap())
            .collect::<Vec<EventIdentifier>>();

        assert_eq!(fetched_moonlight_tx.len(), 9);

        fetched_moonlight_tx.sort_by_key(|mtx| mtx.origin().clone());
        fetched_moonlight_tx.dedup();
        assert_eq!(fetched_moonlight_tx.len(), 6);

        let mut fetched_events = archive
            .full_moonlight_history(pk, None, None, None, None)
            .unwrap()
            .unwrap();
        assert_eq!(fetched_events.len(), 6);

        let fetched_events_reverse_order = archive
            .full_moonlight_history(
                pk,
                Some(super::Order::Descending),
                None,
                None,
                None,
            )
            .unwrap()
            .unwrap();

        assert_eq!(fetched_events_reverse_order.len(), 6);

        fetched_events.reverse();
        assert_eq!(fetched_events_reverse_order, fetched_events,);
    }
}
