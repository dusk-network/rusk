// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

/// Constant depth of the merkle tree that provides the opening proofs.
pub const POSEIDON_TREE_DEPTH: usize = 17;

/// Label used for the ZK transcript initialization. Must be the same for prover
/// and verifier.
pub const TRANSCRIPT_LABEL: &[u8] = b"dusk-network";

mod error;
mod execute;
mod gadgets;
mod send_to_contract_obfuscated;
mod send_to_contract_transparent;
mod types;
mod withdraw_from_obfuscated;
mod withdraw_from_transparent;

pub use error::Error;
pub use execute::*;
pub use send_to_contract_obfuscated::{
    SendToContractObfuscatedCircuit, StcoCrossover, StcoMessage,
};
pub use send_to_contract_transparent::SendToContractTransparentCircuit;
pub use types::DeriveKey;
pub use withdraw_from_obfuscated::{
    WfoChange, WfoCommitment, WithdrawFromObfuscatedCircuit,
};
pub use withdraw_from_transparent::WithdrawFromTransparentCircuit;

#[cfg(feature = "builder")]
pub use execute::builder;
