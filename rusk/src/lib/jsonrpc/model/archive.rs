// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Contains data models specific to the archive functionality of the JSON-RPC
//! interface.

use serde::{Deserialize, Serialize};

/// Represents an event retrieved from the archive.
///
/// This struct mirrors the structure used internally by the node's archive
/// component (`node::archive::sqlite::data::ArchivedEvent`) but is adapted for
/// the JSON-RPC layer.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ArchivedEvent {
    /// The transaction hash or origin identifier associated with the event.
    pub origin: String,
    /// The topic categorizing the event (e.g., "moonlight", "stake").
    pub topic: String,
    /// The source identifier, typically a contract ID, that emitted the event.
    pub source: String,
    /// The raw event data payload.
    #[serde(with = "super::serde_helper::base64_vec_u8")]
    pub data: Vec<u8>,
}
