// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

/// Defines wire frame schema.
#[derive(Debug, Default)]
pub struct Frame {
    // Header fields
    version: [u8; 8],
    reserved: u64,
    checksum: [u8; 4],
    msg_topic: u8,
}
