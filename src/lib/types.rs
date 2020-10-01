// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

/// This struct is here as a placeholder, until we have an actual
/// BN256 implementation.
/// The reason the point is 129 bytes long, is because on the Golang
/// side, the BN256 implementation produces points with 129 bytes.
/// This may change later down the line.
pub struct BN256Point([u8; 129]);

impl BN256Point {
    /// Create a BN256Point from an array of bytes.
    pub fn from_bytes(bytes: [u8; 129]) -> Self {
        BN256Point(bytes)
    }

    /// Return the underlying array of bytes.
    pub fn to_bytes(&self) -> [u8; 129] {
        self.0
    }
}
