// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//#![allow(non_snake_case)]
//pub mod dusk_contract;
pub mod gadgets;

#[cfg(test)]
pub(crate) mod leaf;

mod execute;

pub use execute::ExecuteCircuit;
