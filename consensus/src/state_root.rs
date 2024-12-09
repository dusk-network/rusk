// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bytecheck::CheckBytes;
use execution_core::CommitRoot;
use rkyv::{Archive, Deserialize, Serialize};
use std::borrow::Borrow;

use crate::merkle::Hash;

#[derive(
    Debug,
    Copy,
    Clone,
    Archive,
    Deserialize,
    Serialize,
    PartialOrd,
    Ord,
    PartialEq,
    Eq,
)]
#[archive_attr(derive(CheckBytes))]
pub struct StateRoot(Hash);

impl StateRoot {
    pub fn from(h: Hash) -> Self {
        Self(h)
    }
    pub fn from_bytes(a: [u8; 32]) -> Self {
        Self(Hash::from(a))
    }
    pub fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }
    pub fn as_commit_root(&self) -> CommitRoot {
        CommitRoot::from_bytes(*self.0.as_bytes())
    }
    pub fn from_commit_root<T: Borrow<CommitRoot>>(commit_root: T) -> Self {
        StateRoot::from_bytes(*commit_root.borrow().as_bytes())
    }
}
