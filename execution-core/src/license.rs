// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Types used by Dusk's license contract.

use crate::{reserved, ContractId};

/// ID of the genesis license contract
pub const LICENSE_CONTRACT: ContractId = reserved(0x3);

/// The depth of the merkle tree of licenses stored in the license-contract.
pub const LICENSE_TREE_DEPTH: usize = 17;
/// The arity of the merkle tree of licenses stored in the
/// license-contract.
pub use poseidon_merkle::ARITY as LICENSE_TREE_ARITY;
/// The merkle tree of licenses stored in the license-contract.
pub type LicenseTree = poseidon_merkle::Tree<(), LICENSE_TREE_DEPTH>;
/// The merkle opening for a license-hash in the merkle tree of licenses.
pub type LicenseOpening = poseidon_merkle::Opening<(), LICENSE_TREE_DEPTH>;
/// the tree item for the merkle-tree of licenses stored in the
/// license-contract.
pub type LicenseTreeItem = poseidon_merkle::Item<()>;
