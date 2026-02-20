// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::OnceLock;

use dusk_core::signatures::bls::BlsVersion;

/// Activation height value that means "never activate".
const NEVER: u64 = u64::MAX;

/// Active protocol hardfork.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum HardFork {
    /// Behavior before any explicit hardfork activation.
    PreFork,
    /// Behavior after Aegis activation.
    Aegis,
}

impl HardFork {
    /// Returns the BLS signature version for this hardfork.
    pub fn bls_version(&self) -> BlsVersion {
        match self {
            HardFork::Aegis => BlsVersion::V2,
            HardFork::PreFork => BlsVersion::V1,
        }
    }
}

/// Returns the BLS version for the given block height.
pub fn bls_version_at(block_height: u64) -> BlsVersion {
    hard_fork_at(block_height).bls_version()
}

static AEGIS_ACTIVATION_HEIGHT: OnceLock<u64> = OnceLock::new();

/// Initializes the Aegis activation height once for this process.
pub fn set_aegis_activation_height(block_height: u64) {
    if let Some(existing) = AEGIS_ACTIVATION_HEIGHT.get() {
        debug_assert_eq!(
            *existing, block_height,
            "Aegis activation height changed after initialization"
        );
        return;
    }

    let _ = AEGIS_ACTIVATION_HEIGHT.set(block_height);
}

/// Returns the configured Aegis activation height, or `NEVER` if unset.
fn aegis_activation_height() -> u64 {
    *AEGIS_ACTIVATION_HEIGHT.get().unwrap_or(&NEVER)
}

/// Returns the active hardfork for `block_height`.
pub fn hard_fork_at(block_height: u64) -> HardFork {
    if block_height >= aegis_activation_height() {
        HardFork::Aegis
    } else {
        HardFork::PreFork
    }
}

/// Returns the active hardfork for `block_height`, given an activation height.
#[cfg(test)]
pub(crate) const fn hard_fork_at_with_activation(
    block_height: u64,
    aegis_activation_height: u64,
) -> HardFork {
    if block_height >= aegis_activation_height {
        HardFork::Aegis
    } else {
        HardFork::PreFork
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aegis_activation_boundary() {
        assert_eq!(hard_fork_at_with_activation(99, 100), HardFork::PreFork);
        assert_eq!(hard_fork_at_with_activation(100, 100), HardFork::Aegis);
        assert_eq!(hard_fork_at_with_activation(101, 100), HardFork::Aegis);
        assert_eq!(hard_fork_at_with_activation(101, NEVER), HardFork::PreFork);
    }
}
