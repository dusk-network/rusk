// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod error;
mod execute;
mod gadgets;
mod send_to_contract_obfuscated;
mod send_to_contract_transparent;
mod withdraw_from_obfuscated;
mod withdraw_from_transparent;

/// Label used for the ZK transcript initialization. Must be the same for prover
/// and verifier.
pub const TRANSCRIPT_LABEL: &[u8] = b"dusk-network";

pub use error::Error;
pub use execute::*;
pub use send_to_contract_obfuscated::SendToContractObfuscatedCircuit;
pub use send_to_contract_transparent::SendToContractTransparentCircuit;
pub use withdraw_from_obfuscated::WithdrawFromObfuscatedCircuit;
pub use withdraw_from_transparent::WithdrawFromTransparentCircuit;

#[cfg(any(test, feature = "builder"))]
pub use execute::builder;
