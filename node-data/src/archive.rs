// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::events::contract::ContractTxEvent;
use crate::ledger::Hash;

/// Defined data, that the archivist will store.
///
/// This is also the type of the mpsc channel where the archivist listens for
/// data to archive.
///
/// Any data that archive nodes can store must be defined here
#[derive(Debug)]
pub enum ArchivalData {
    /// List of contract events from one block together with the block height
    /// and block hash.
    ArchivedEvents(u64, Hash, Vec<ContractTxEvent>),
}

impl ArchivalData {
    /// Returns the block height of the data.
    pub fn block_height(&self) -> u64 {
        match self {
            ArchivalData::ArchivedEvents(height, _, _) => *height,
        }
    }

    /// Returns the block hash of the data.
    pub fn block_hash(&self) -> &Hash {
        match self {
            ArchivalData::ArchivedEvents(_, hash, _) => hash,
        }
    }
}
