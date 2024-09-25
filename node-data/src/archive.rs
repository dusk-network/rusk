// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::events::contract::ContractTxEvent;
use crate::ledger::Hash;

type HexHash = String;

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
    FinalizedBlock(u64, HexHash),
    DeletedBlock(u64, HexHash),
}
