// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::{Candidate, Ledger, Persist, Registry, DB};
use anyhow::{Context, Result};

use node_common::encoding::*;
use node_common::ledger;
use node_common::Serializable;

use crate::database::Mempool;

use dusk_bytes::Serializable as DuskBytesSerializable;

use rocksdb_lib::{
    ColumnFamily, ColumnFamilyDescriptor, DBAccess, DBCommon, DBWithThreadMode,
    IteratorMode, MultiThreaded, OptimisticTransactionDB,
    OptimisticTransactionOptions, Options, ReadOptions, SnapshotWithThreadMode,
    WriteOptions,
};

use std::io::Read;
use std::{marker::PhantomData, path::Path, sync::Arc};
use tokio::io::AsyncWriteExt;

enum TxType {
    ReadWrite,
    ReadOnly,
}

const CF_LEDGER: &str = "cf_ledger";
const CF_CANDIDATES: &str = "cf_candidates";
const CF_MEMPOOL: &str = "cf_mempool";
const CF_MEMPOOL_NULLIFIERS: &str = "cf_mempool_nullifiers";
const CF_MEMPOOL_FEES: &str = "cf_mempool_fees";

const MAX_MEMPOOL_SIZE: usize = 64 * 1024 * 1024; // 64 MiB

pub struct Backend {
    rocksdb: Arc<OptimisticTransactionDB>,
}

impl Backend {
    fn begin_tx(
        &self,
        access_type: TxType,
    ) -> DBTransaction<'_, OptimisticTransactionDB> {
        // Create a new RocksDB transaction
        let write_options = WriteOptions::default();
        let mut tx_options = OptimisticTransactionOptions::default();

        let inner = self.rocksdb.transaction_opt(&write_options, &tx_options);

        // Borrow column families
        let ledger_cf = self
            .rocksdb
            .cf_handle(CF_LEDGER)
            .expect("ledger column family must exist");

        let candidates_cf = self
            .rocksdb
            .cf_handle(CF_CANDIDATES)
            .expect("candidates column family must exist");

        let mempool_cf = self
            .rocksdb
            .cf_handle(CF_MEMPOOL)
            .expect("mempool column family must exist");

        let nullifiers_cf = self
            .rocksdb
            .cf_handle(CF_MEMPOOL_NULLIFIERS)
            .expect("CF_MEMPOOL_NULLIFIERS column family must exist");

        let fees_cf = self
            .rocksdb
            .cf_handle(CF_MEMPOOL_FEES)
            .expect("CF_MEMPOOL_FEES column family must exist");

        let snapshot = self.rocksdb.snapshot();

        DBTransaction::<'_, OptimisticTransactionDB> {
            inner,
            access_type,
            candidates_cf,
            ledger_cf,
            mempool_cf,
            nullifiers_cf,
            fees_cf,
            snapshot,
        }
    }
}

impl DB for Backend {
    type P<'a> = DBTransaction<'a, OptimisticTransactionDB>;

    fn create_or_open(path: String) -> Self {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        opts.set_level_compaction_dynamic_level_bytes(true);

        // Configure CF_MEMPOOL column family so it benefits from low
        // write-latency of L0
        let mut mp_opts = Options::default();
        mp_opts.set_write_buffer_size(MAX_MEMPOOL_SIZE);

        // Disable WAL by default
        mp_opts.set_manual_wal_flush(true);

        // Disable flush-to-disk by default
        mp_opts.set_disable_auto_compactions(true);

        let cfs = vec![
            ColumnFamilyDescriptor::new(CF_LEDGER, Options::default()),
            ColumnFamilyDescriptor::new(CF_CANDIDATES, Options::default()),
            ColumnFamilyDescriptor::new(CF_MEMPOOL, mp_opts.clone()),
            ColumnFamilyDescriptor::new(CF_MEMPOOL_NULLIFIERS, mp_opts.clone()),
            ColumnFamilyDescriptor::new(CF_MEMPOOL_FEES, mp_opts),
        ];

        Self {
            rocksdb: Arc::new(
                rocksdb_lib::OptimisticTransactionDB::open_cf_descriptors(
                    &opts,
                    Path::new(&path),
                    cfs,
                )
                .expect("should be a valid database"),
            ),
        }
    }

    fn view<F>(&self, f: F) -> Result<()>
    where
        F: for<'a> FnOnce(Self::P<'a>) -> Result<()>,
    {
        // Create a new read-only transaction
        let tx = self.begin_tx(TxType::ReadOnly);

        // Execute all read-only transactions in isolation
        f(tx)?;

        Ok(())
    }

    fn update<F>(&self, execute: F) -> Result<()>
    where
        F: for<'a> FnOnce(&Self::P<'a>) -> Result<()>,
    {
        // Create read-write transaction
        let tx = self.begin_tx(TxType::ReadWrite);

        // If f returns err, no commit will be applied into backend
        // storage
        execute(&tx)?;

        // Apply changes in atomic way
        tx.commit()?;

        Ok(())
    }

    fn close(&mut self) {}
}

pub struct DBTransaction<'db, DB: DBAccess> {
    inner: rocksdb_lib::Transaction<'db, DB>,
    access_type: TxType,

    candidates_cf: &'db ColumnFamily,
    ledger_cf: &'db ColumnFamily,
    mempool_cf: &'db ColumnFamily,
    nullifiers_cf: &'db ColumnFamily,
    fees_cf: &'db ColumnFamily,

    snapshot: SnapshotWithThreadMode<'db, DB>,
}

impl<'db, DB: DBAccess> Ledger for DBTransaction<'db, DB> {
    fn store_block(&self, b: &ledger::Block, persisted: bool) -> Result<()> {
        let mut serialized = vec![];
        b.header.write(&mut serialized)?;

        self.inner
            .put_cf(self.ledger_cf, b.header.hash, serialized)?;

        Ok(())
    }

    fn delete_block(&self, b: &ledger::Block) -> Result<()> {
        let key = b.header.hash;
        self.inner.delete_cf(self.ledger_cf, key)?;

        Ok(())
    }

    fn fetch_block(&self, hash: &[u8]) -> Result<Option<ledger::Block>> {
        if let Some(blob) = self.snapshot.get_cf(self.ledger_cf, hash)? {
            let b = ledger::Block::read(&mut &blob[..])?;
            return Ok(Some(b));
        }

        // Block not found
        Ok(None)
    }
}

impl<'db, DB: DBAccess> Candidate for DBTransaction<'db, DB> {
    fn store_candidate_block(&self, b: ledger::Block) -> Result<()> {
        let mut serialized = vec![];
        b.write(&mut serialized)?;

        self.inner
            .put_cf(self.candidates_cf, b.header.hash, serialized)?;

        Ok(())
    }

    fn fetch_candidate_block(
        &self,
        hash: &[u8],
    ) -> Result<Option<ledger::Block>> {
        if let Some(blob) = self.snapshot.get_cf(self.candidates_cf, hash)? {
            let b = ledger::Block::read(&mut &blob[..])?;
            return Ok(Some(b));
        }

        // Block not found
        Ok(None)
    }

    /// Deletes all items from CF_CANDIDATES column family
    fn clear_candidates(&self) -> Result<()> {
        let iter = self
            .inner
            .iterator_cf(self.candidates_cf, IteratorMode::Start);

        // Iterate through the CF_CANDIDATES column family and delete all items
        iter.map(Result::unwrap)
            .map(|(key, _)| {
                self.inner.delete_cf(self.candidates_cf, key);
            })
            .collect::<Vec<_>>();

        Ok(())
    }
}

impl<'db, DB: DBAccess> Persist for DBTransaction<'db, DB> {
    /// Deletes all items from both CF_LEDGER and CF_CANDIDATES column families
    fn clear_database(&self) -> Result<()> {
        // Create an iterator over the column family CF_LEDGER
        let iter = self.inner.iterator_cf(self.ledger_cf, IteratorMode::Start);

        // Iterate through the CF_LEDGER column family and delete all items
        iter.map(Result::unwrap)
            .map(|(key, _)| {
                self.inner.delete_cf(self.ledger_cf, key);
            })
            .collect::<Vec<_>>();

        self.clear_candidates()?;
        Ok(())
    }

    fn commit(self) -> Result<()> {
        if let Err(e) = self.inner.commit() {
            return Err(anyhow::Error::new(e).context("failed to commit"));
        }

        Ok(())
    }
}

impl<'db, DB: DBAccess> Mempool for DBTransaction<'db, DB> {
    fn add_tx(&self, tx: &ledger::Transaction) -> Result<()> {
        // Map Hash to serialized transaction
        let mut d = vec![];
        tx.write(&mut d)?;

        let hash = tx.hash();
        self.inner.put_cf(self.mempool_cf, hash, d)?;

        // Add Secondary indexes //
        // Nullifiers
        for n in tx.inner.inputs().into_iter() {
            let key: [u8; 32] = n.to_bytes().into();
            self.inner.put_cf(self.nullifiers_cf, key, vec![0])?;
        }

        // Map Fee_Hash to Null to facilitate sort-by-fee
        self.inner.put_cf(
            self.fees_cf,
            serialize_fee_key(tx.gas_price(), hash)?,
            vec![0],
        )?;

        Ok(())
    }

    fn get_tx(&self, hash: [u8; 32]) -> Result<Option<ledger::Transaction>> {
        let data = self.inner.get_cf(self.mempool_cf, hash)?;

        match data {
            // None has a meaning key not found
            None => Ok(None),
            Some(blob) => {
                Ok(Some(ledger::Transaction::read(&mut &blob.to_vec()[..])?))
            }
        }
    }

    fn get_tx_exists(&self, h: [u8; 32]) -> bool {
        // Check for hash if exists without deserializing
        self.snapshot.get_cf(self.mempool_cf, h).is_ok()
    }

    fn delete_tx(&self, h: [u8; 32]) -> Result<bool> {
        let tx = self.get_tx(h)?;
        if let Some(tx) = tx {
            let hash = tx.hash();

            self.inner.delete_cf(self.mempool_cf, hash)?;

            // Delete Secondary indexes
            // Delete Nullifiers
            for n in tx.inner.inputs().into_iter() {
                let key: [u8; 32] = n.to_bytes().into();
                self.inner.delete_cf(self.nullifiers_cf, key)?;
            }

            // Delete Fee_Hash
            self.inner.delete_cf(
                self.fees_cf,
                serialize_fee_key(tx.gas_price(), hash)?,
            )?;

            return Ok(true);
        }

        Ok(false)
    }

    fn get_any_nullifier_exists(&self, nullifiers: Vec<[u8; 32]>) -> bool {
        nullifiers
            .into_iter()
            .all(|n| self.snapshot.get_cf(self.nullifiers_cf, n).is_ok())
    }

    fn get_txs_sorted_by_fee(
        &self,
        max_gas_limit: u64,
    ) -> Result<Vec<Option<ledger::Transaction>>> {
        // The slippageGasLimit is the threshold that consider the "estimated
        // gas spent" acceptable even if it exceeds the strict GasLimit.
        // This is required to avoid to iterate the whole mempool until
        // it fit perfectly the block GasLimit
        let slippage_gas_limit = max_gas_limit + max_gas_limit / 10;

        let mut iter = self.inner.raw_iterator_cf(self.fees_cf);
        iter.seek_to_last();

        let mut total_gas: u64 = 0;
        let mut txs_list = vec![];

        // Iterate all keys from the end in reverse lexicographic order
        while iter.valid() {
            if let Some(key) = iter.key() {
                let (read_gp, tx_hash) =
                    deserialize_fee_key(&mut &key.to_vec()[..])?;

                let mut tx = self.get_tx(tx_hash)?;
                if let Some(tx) = tx {
                    let gas_price = tx.gas_price();
                    debug_assert_eq!(read_gp, gas_price);
                    total_gas += gas_price;

                    if total_gas > slippage_gas_limit {
                        break;
                    }

                    txs_list.push(Some(tx));
                }
            }

            iter.prev();
        }

        Ok(txs_list)
    }
}

fn serialize_fee_key(fee: u64, hash: [u8; 32]) -> std::io::Result<Vec<u8>> {
    let mut w = vec![];
    std::io::Write::write_all(&mut w, &fee.to_be_bytes())?;
    std::io::Write::write_all(&mut w, &hash)?;
    Ok(w)
}

fn deserialize_fee_key<R: Read>(r: &mut R) -> Result<(u64, [u8; 32])> {
    // Read fee
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    let fee = u64::from_be_bytes(buf);

    // Read tx hash
    let mut hash = [0u8; 32];
    r.read_exact(&mut hash[..])?;

    Ok((fee, hash))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::prelude::*;
    use rand::Rng;

    #[test]
    fn test_store_block() {
        let t = TestWrapper {
            path: "_test_store_block",
        };

        t.run(|path| {
            let db: Backend = Backend::create_or_open(path.to_owned());

            let b = mock_block(100);
            let hash = b.header.hash;

            assert!(db
                .update(|txn| {
                    txn.store_block(&b, false)?;
                    Ok(())
                })
                .is_ok());

            db.view(|txn| {
                assert_eq!(
                    txn.fetch_block(&hash)?.unwrap().header.hash,
                    b.header.hash
                );
                Ok(())
            });

            assert!(db
                .update(|txn| {
                    txn.clear_database()?;
                    Ok(())
                })
                .is_ok());

            db.view(|txn| {
                assert!(txn.fetch_block(&hash)?.is_none());
                Ok(())
            });
        });
    }

    #[test]
    fn test_read_only() {
        let t = TestWrapper {
            path: "_test_read_only",
        };

        t.run(|path| {
            let db: Backend = Backend::create_or_open(path.to_owned());
            let b = mock_block(22);
            assert!(db
                .view(|txn| {
                    txn.store_block(&b, false)?;
                    Ok(())
                })
                .is_ok());

            db.view(|txn| {
                assert!(txn.fetch_block(&b.header.hash)?.is_none());
                Ok(())
            });
        });
    }

    #[test]
    fn test_transaction_isolation() {
        let t = TestWrapper {
            path: "_test_transaction_isolation",
        };

        t.run(|path| {
            let db: Backend = Backend::create_or_open(path.to_owned());
            let mut b = mock_block(101);
            let hash = b.header.hash;

            db.view(|txn| {
                // Simulate a concurrent update is committed during read-only
                // transaction
                assert!(db
                    .update(|txn| {
                        txn.store_block(&b, false)?;

                        // No need to support Read-Your-Own-Writes
                        assert!(txn.fetch_block(&hash)?.is_none());
                        Ok(())
                    })
                    .is_ok());

                // Asserts that the read-only/view transaction runs in isolation
                assert!(txn.fetch_block(&hash)?.is_none());
                Ok(())
            });

            // Asserts that update was done
            db.view(|txn| {
                assert_blocks_eq(&mut txn.fetch_block(&hash)?.unwrap(), &mut b);
                Ok(())
            });
        });
    }

    fn assert_blocks_eq(a: &mut ledger::Block, b: &mut ledger::Block) {
        assert!(a.calculate_hash().is_ok());
        assert!(b.calculate_hash().is_ok());
        assert!(a.header.hash.eq(&b.header.hash));
    }

    #[test]
    fn test_add_tx() {
        let t = TestWrapper {
            path: "test_add_tx",
        };

        t.run(|path| {
            let db: Backend = Backend::create_or_open(path.to_owned());
            let t = mock_tx(1, 1);

            assert!(db.update(|txn| { txn.add_tx(&t) }).is_ok());

            db.view(|txn| {
                assert!(txn.get_tx_exists(t.hash()));

                let fetched_tx =
                    txn.get_tx(t.hash()).expect("valid contract call").unwrap();

                assert_eq!(
                    fetched_tx.hash(),
                    t.hash(),
                    "fetched transaction should be the same"
                );
                Ok(())
            });

            // Delete a contract call
            db.update(|txn| {
                assert!(txn.delete_tx(t.hash()).expect("valid tx"));
                Ok(())
            });
        });
    }

    #[test]
    fn test_tx_sorted_by_fee() {
        let t = TestWrapper {
            path: "test_tx_sorted_by_fee",
        };

        t.run(|path| {
            let db: Backend = Backend::create_or_open(path.to_owned());
            // Populate mempool with N contract calls
            let mut rng = rand::thread_rng();
            db.update(|txn| {
                for i in 0..10u32 {
                    txn.add_tx(&mock_tx(i, rng.gen::<u32>() as u64))?;
                }
                Ok(())
            });

            // Assert txs are retrieved in descending order sorted by fee
            let max_gas_limit = u64::MAX - u64::MAX / 10;
            db.view(|txn| {
                let txs = txn
                    .get_txs_sorted_by_fee(max_gas_limit)
                    .expect("should return all txs");

                assert!(!txs.is_empty());
                let mut last_fee = u64::MAX;
                for t in txs {
                    let fee = t.expect("valid tx").gas_price();
                    assert!(
                        fee <= last_fee,
                        "tx fees are not in decreasing order"
                    );
                }

                Ok(())
            });
        });
    }

    #[test]
    fn test_block_size_limit() {
        let t = TestWrapper {
            path: "test_block_size_limit",
        };

        t.run(|path| {
            let db: Backend = Backend::create_or_open(path.to_owned());

            db.update(|txn| {
                for i in 0..10u32 {
                    txn.add_tx(&mock_tx(i, i as u64))?;
                }
                Ok(())
            });

            let max_gas_limit: u32 = 9 + 8 + 7;
            db.view(|txn| {
                let txs = txn
                    .get_txs_sorted_by_fee(max_gas_limit as u64)
                    .expect("should return all txs");

                assert_eq!(txs.len(), 3);
                Ok(())
            });
        });
    }

    struct TestWrapper {
        path: &'static str,
    }

    impl TestWrapper {
        pub fn run<F>(&self, test_func: F)
        where
            F: FnOnce(&str),
        {
            // Destroy/deletion of a database can happen only before creating
            let opts = Options::default();
            rocksdb_lib::DB::destroy(&opts, Path::new(&self.path));

            test_func(self.path);

            // Destroy/deletion of a database can happen only after dropping DB.
            let opts = Options::default();
            rocksdb_lib::DB::destroy(&opts, Path::new(&self.path));
        }
    }

    fn mock_block(height: u64) -> ledger::Block {
        ledger::Block::new(
            ledger::Header {
                version: 0,
                height,
                timestamp: 11112222,
                gas_limit: 123456,
                prev_block_hash: [10; 32],
                seed: ledger::Signature::default(),
                generator_bls_pubkey: ledger::BlsPubkey([12; 96]),
                state_hash: [13; 32],
                hash: [0; 32],
                cert: ledger::Certificate::default(),
            },
            vec![],
        )
        .expect("should be valid hash")
    }

    fn mock_tx(h: u32, gas_price: u64) -> ledger::Transaction {
        let mut hash = [0u8; 32];
        hash[..4].copy_from_slice(&h.to_be_bytes());

        let fixed = "010000000000000001020304050607080102030405060708010203040506070801020304050607080200000000000000010c8088b9e8c9d06915673d4d94fc76348fb7ce7503e8587f30caea67ab8379b815ce6aba274054f337bdd92d9411d8be3f282b05e3c6d42e8eea9f3215b8de33b96a3c7c1dbcb4d8cdd8ef13e50e84cf6480116311677676269d3e662cea608c5a3479e042102a78621252a37f2d99e6824e17a2b11597147d1adf4624e7d436ffffffffffffffff997ebe7877346dc48137c1d115176c60c5dbf0ea77dd8cdca0cfbc0f3d90304ecb5b2b3d60a2b9d4df4a999ef3a768f8bd75c75aac343bff35bed7cfb2e3513315e8ece73c24ca0c97bda403149dcf9fea1c8827b682c1bbe089c8d10355c45e01e549d068cb470cbefe6fddd3b2d8aacfa5a76805e725d5394e882a79d157695ec48dcb7e531ccc3b334ae122d4fd40e242e7d8a85fdb82bd4c9e9621a9a60d042dbbaec8a2acb879b48d311f1264b1aafe6bf26ccc0bb250af7a2e19e8dcdc3851f382c509fb449a701a93c9489ae97bae88feaebe38fc6c128dc4b286724c10ffffffffffffffff14b611da24f94e89dd03121410f05b53c52cfc785da3c8d59bb65d07b78ab0241c6a8f3ffadc251790b9f78a31b82246c883cbfa1330337bd094045c01dcca2a7de1eadf6f1f7116169ed9dd10541a407035bb8fe834a973d8176f51f07a8435fee6a01aa94b675366ed1b054b8091542329dd1538bcec8a7503906281f0b61200ca9a3b00000000GASPRICEd85dbd596fc0476c779f3e2e7b5e58b732cb71f9ca056a8828cf845885a22f17848a28b1224942eb4222b7c43fc01e60529c7ee5fab115f3802c91179d0edfa19851d4394c5da06a86f955b2bd1305672e61a9569b5e024f03c957d4160d3d23fad4651d0897d60d89845c58baee90dbb291366e711628910673b9f3eedaaec355d87e2b2619a6809157bf01d3579145794a2b10e5e0f23d053e48a699ad318d80d2e737ca67e32f0848724907f3a847befe125d83031fc249cc24d489bee3cca6dfba0129d5578102c594b72631a13797cc0413391a5a1886c7536e6fdc0c489dfdbc00baba13e05157a7ab7273523dbb98d34c06e3a058424f361aad4a8fbda04b3327dbf973a2fc07d54445ebe6651b2e35a3f5c983dad6f05599505d20e8049ab8b6a8f099304dbc4badb806e2e8b02f90619eacef17710c48c316cddd0889badea8613806d13450208797859e6271335cda185bbfc5844358e701c0ca03ad84e86019661d4b29336d10be7f2d1510cb65478f0ea3e0baea5d49ff962bcccdcf4396a0b3cfed0f1b8c5537b148f88f31e782f30be64807cad8900706b18a31cce9a743694b0abf94d6ff32789e870b3b70970bc2a01b69faea5a6dfc3514b4d6cf831dd715429cb3c9c3c9011422260233eab35f30dec5415fe06f9a22e5e4847cde93f61e896ebeec082ced1e65b7bf5dfe6f6dd064d2649580ae5ec6b09934167cdd0efc24150dee406c18dc4d6def110c74049a3f14c7d2b019606518ab91cba648915908d032c33cd3a6c07bfb908902c5a8bd55ed5fb25582659a9f4fb82aedba03c6946823b020ff8fad039772696c1b58a3434a5c53f5b6670943e90ccf49fb24d88929f467341cd68978082969dfc75ccdf161e1340bb3d66633b52703b2efd6cf769395fa892f5738cf5dee96afe27fe085bed54dd607bc0f0b3fe5fd5e83f1a18ed9e3457ac28bc6a49224c20f17d63fbc38f2d3e49af4f108407a9523e55fc1e89a2c221b0d15a993a3856a9f9618655555f7828734da3193ad2353c81a6f0720e90dbc62a8dcdd1e117b8f6addd574a6c483a5bebb06255e9614ff22ce4ac848de8ee8df47bd133fbd5f46bf9bf9a56e80d6e411cf2803186dad1a7cd9176ba85dff17e29471fb1c6f3a9304630e190406857e511c93711eca6a472f89005ddef430f0df953dcf5a3751bddaf39da32e25a87b1f41cc23f14b25ea9e0289785520696b0a82d6a23a19eb11ca32021c414ba83f0d4012933a4a962826e7185f21f440c8b08c1adf58aec9daee1c8e15e607239e819fc5dea80c697e800a1a18acd235789fb9dfee43f3e8a51ba190656ca8ee9dc7ed1cbfce26a0deb7563f52292f3f6bef6360095b1fa416afa01640ddbabbd3b8fc15223d50c0cdc80cb846947b80408764fab356051d2783e2a9e54917cfaab223c75dd8d5187841fbe93fc79bbc1d63ffffce68ae16c3b4ef3bd92d87bec21f2f958ab4f91535f10c50ef186e3a4d2a43b8060ac15b9ef21256e52123862563540c14d9d0904c20c70d2c5915e352b582f7ee0dfe3338658c1e7245b651428799705d9b76847e9fc8a872ef3aae9c978ca64e3f5f11dd7d49decaad5c299680e7478ddc9651d8578774431b46cc701601af616f9c7323ce76fcd1c6055f7d02652c9a2354ad21ebfd1df37d5254609e3d38666940a2a6dd21c59400bf444f8b297203243de4099b1c8640fb43849f160cdab42a52e0a107df5db400819f7587957f07d72cb498ae97aa6d1e67ae2900ff56f7378f742e04fcdedd2a72ef20aea340f9f65cff2bedc1362733170906a443a1964bdc59c245808014604e2fc9c9f23ecc590da6bedcb81c69ef8f369d69a0c9c663e0faccefde8bf848224166c59b49eb9a58f8fb38bdb42f6b33b5470378bfe21a980b1d78a8da4c32b4f380127bdd6a9c0c96f1b3ee4c0bbc69fa312e7a77560ad2eafdc97017ff9e51da30ee8e2acfaef091236c4c6cf66e2f43129d70744812d2eafdc97017ff9e51da30ee8e2acfaef091236c4c6cf66e2f43129d707448126981ddc905c11356d461b7ccc828dc1ac8e3c92cc9ba3619ee76f9150095a75304d64fd0d2d436f18e6881aae6b7d99bed17078b8f508f0cf4bb2dbd3e7f7871170c739f9d9ea4404bff4066c3ed34d6a52245965b485b766344a380f65e5d2800000000000000000000000000000000";

        let utx_bytes = hex::decode(fixed.replace(
            "GASPRICE",
            hex::encode((gas_price).to_le_bytes()).as_str(),
        ))
        .expect("decodable data");

        ledger::Transaction {
            inner: dusk_wallet_core::Transaction::from_slice(&utx_bytes)
                .expect("should be valid"),
            gas_spent: None,
        }
    }
}
