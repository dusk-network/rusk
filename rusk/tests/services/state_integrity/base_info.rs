// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bytecheck::CheckBytes;
use dusk_core::abi::ContractId;
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Debug, Clone, Default, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct BaseInfo {
    pub contract_hints: Vec<ContractId>,
    pub maybe_base: Option<[u8; 32]>,
    pub level: u64,
}
