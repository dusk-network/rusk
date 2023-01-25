// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_consensus::commons::Block;
pub mod rocksdb;
use anyhow::Result;

pub trait DB: Send + Sync + 'static {
    type T<'a>: Tx;

    /// Creates or open a database located at this path.
    ///
    /// Panics if opening db or creating one fails.
    fn create_or_open(path: String) -> Self;

    /// Provides a managed execution of a read-only isolated transaction.
    fn view<F>(&self, f: F) -> Result<()>
    where
        F: for<'a> FnOnce(Self::T<'a>) -> Result<()>;

    /// Provides a managed execution of a read-write atomic transaction.
    ///
    /// An atomic transaction is an indivisible and irreducible series of
    /// database operations such that either all occur, or nothing occurs.
    ///
    /// Transaction commit will happen only if no error is returned by `fn`
    /// and no panic is raised on `fn` execution.
    fn update<F>(&self, f: F) -> Result<()>
    where
        F: for<'a> FnOnce(&Self::T<'a>) -> Result<()>;

    fn close(&mut self);
}

/// Implements both read-write and read-only transactions to DB.
pub trait Tx {
    // Read-write transactions
    fn store_block(&self, b: &Block, persisted: bool) -> Result<()>;
    fn delete_block(&self, b: &Block) -> Result<()>;
    fn fetch_block(&self, hash: &[u8]) -> Result<Option<Block>>;

    // Candidate block functions
    fn store_candidate_block(&self, cm: Block) -> Result<()>;
    fn fetch_candidate_block(&self, hash: &[u8]) -> Result<Option<Block>>;
    fn clear_candidates(&self) -> Result<()>;

    fn clear_database(&self) -> Result<()>;

    fn commit(self) -> Result<()>;
}

#[derive(Default)]
pub struct Registry {
    pub tip_hash: [u8; 32],
    pub persisted_hash: [u8; 32],
}
