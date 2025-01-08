// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::services::state_integrity::hash::Hash;
use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

// There are max `2^16` pages in a 32-bit memory
const P32_HEIGHT: usize = 8;
pub const P32_ARITY: usize = 4;

type PageTree32 = dusk_merkle::Tree<[u8; 32], P32_HEIGHT, P32_ARITY>;

// There are max `2^26` pages in a 64-bit memory
const P64_HEIGHT: usize = 13;
const P64_ARITY: usize = 4;

type PageTree64 = dusk_merkle::Tree<[u8; 32], P64_HEIGHT, P64_ARITY>;

// This means we have max `2^32` contracts
const C_HEIGHT: usize = 32;
pub const C_ARITY: usize = 2;

#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub enum PageTree {
    Wasm32(PageTree32),
    Wasm64(PageTree64),
}

pub type ContractMemTree = dusk_merkle::Tree<Hash, C_HEIGHT, C_ARITY>;
