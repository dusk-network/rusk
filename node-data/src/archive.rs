// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

type HexHash = String;

/// Defined data, that the archivist will store.
///
/// This is also the type of the mpsc channel where the archivist listens for
/// data to archive. ContractEvents together with an unfinalized Block are
/// archived directly and not stored here.
#[derive(Debug)]
pub enum ArchivalData {
    FinalizedBlock(u64, HexHash),
    DeletedBlock(u64, HexHash),
}
