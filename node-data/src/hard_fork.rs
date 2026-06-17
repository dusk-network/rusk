// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::OnceLock;

use dusk_core::signatures::bls::BlsVersion;
use dusk_core::transfer::TransactionFormat;

/// Activation height value that means "never activate".
const NEVER: u64 = u64::MAX;

/// Active protocol hardfork.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum HardFork {
    /// Behavior before any explicit hardfork activation.
    PreFork,
    /// Behavior after Aegis activation.
    Aegis,
    /// Behavior after Boreas activation.
    Boreas,
}

impl HardFork {
    /// Returns the BLS signature version for this hardfork.
    pub const fn bls_version(self) -> BlsVersion {
        match self {
            HardFork::Aegis | HardFork::Boreas => BlsVersion::V2,
            HardFork::PreFork => BlsVersion::V1,
        }
    }

    /// Returns the ledger transaction format for this hardfork era.
    pub const fn ledger_tx_format(self) -> TransactionFormat {
        match self {
            HardFork::PreFork => TransactionFormat::PreAegis,
            HardFork::Aegis => TransactionFormat::Aegis,
            HardFork::Boreas => TransactionFormat::Boreas,
        }
    }

    /// Returns the ingress transaction format for this hardfork era.
    pub const fn ingress_tx_format(self) -> TransactionFormat {
        match self {
            HardFork::PreFork | HardFork::Aegis => TransactionFormat::Aegis,
            HardFork::Boreas => TransactionFormat::Boreas,
        }
    }
}

/// Returns the BLS version for the given block height.
pub fn bls_version_at(block_height: u64) -> BlsVersion {
    hard_fork_at(block_height).bls_version()
}

static AEGIS_ACTIVATION_HEIGHT: OnceLock<u64> = OnceLock::new();
static BOREAS_ACTIVATION_HEIGHT: OnceLock<u64> = OnceLock::new();

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

/// Initializes the Boreas activation height once for this process.
pub fn set_boreas_activation_height(block_height: u64) {
    if let Some(existing) = BOREAS_ACTIVATION_HEIGHT.get() {
        debug_assert_eq!(
            *existing, block_height,
            "Boreas activation height changed after initialization"
        );
        return;
    }

    let _ = BOREAS_ACTIVATION_HEIGHT.set(block_height);
}

/// Returns the configured Aegis activation height, or `NEVER` if unset.
fn aegis_activation_height() -> u64 {
    *AEGIS_ACTIVATION_HEIGHT.get().unwrap_or(&NEVER)
}

/// Returns the configured Boreas activation height, or `NEVER` if unset.
fn boreas_activation_height() -> u64 {
    *BOREAS_ACTIVATION_HEIGHT.get().unwrap_or(&NEVER)
}

/// Returns the active hardfork for `block_height`.
pub fn hard_fork_at(block_height: u64) -> HardFork {
    if block_height >= boreas_activation_height() {
        HardFork::Boreas
    } else if block_height >= aegis_activation_height() {
        HardFork::Aegis
    } else {
        HardFork::PreFork
    }
}

/// Returns the ledger transaction format for `block_height`.
pub fn ledger_tx_format_at(block_height: u64) -> TransactionFormat {
    hard_fork_at(block_height).ledger_tx_format()
}

/// Returns the ingress transaction format for the target block height.
pub fn ingress_tx_format_at(block_height: u64) -> TransactionFormat {
    hard_fork_at(block_height).ingress_tx_format()
}

/// Returns the active hardfork for `block_height`, given an activation height.
#[cfg(test)]
pub(crate) const fn hard_fork_at_with_activation(
    block_height: u64,
    aegis_activation_height: u64,
    boreas_activation_height: u64,
) -> HardFork {
    if block_height >= boreas_activation_height {
        HardFork::Boreas
    } else if block_height >= aegis_activation_height {
        HardFork::Aegis
    } else {
        HardFork::PreFork
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hard_fork_and_format_policy_matrix() {
        let cases = [
            (
                99,
                100,
                NEVER,
                HardFork::PreFork,
                TransactionFormat::PreAegis,
                TransactionFormat::Aegis,
            ),
            (
                100,
                100,
                NEVER,
                HardFork::Aegis,
                TransactionFormat::Aegis,
                TransactionFormat::Aegis,
            ),
            (
                101,
                NEVER,
                NEVER,
                HardFork::PreFork,
                TransactionFormat::PreAegis,
                TransactionFormat::Aegis,
            ),
            (
                199,
                100,
                200,
                HardFork::Aegis,
                TransactionFormat::Aegis,
                TransactionFormat::Aegis,
            ),
            (
                200,
                100,
                200,
                HardFork::Boreas,
                TransactionFormat::Boreas,
                TransactionFormat::Boreas,
            ),
        ];

        for (
            height,
            aegis_activation_height,
            boreas_activation_height,
            expected_hard_fork,
            expected_ledger_format,
            expected_ingress_format,
        ) in cases
        {
            let hard_fork = hard_fork_at_with_activation(
                height,
                aegis_activation_height,
                boreas_activation_height,
            );

            assert_eq!(hard_fork, expected_hard_fork);
            assert_eq!(hard_fork.ledger_tx_format(), expected_ledger_format);
            assert_eq!(hard_fork.ingress_tx_format(), expected_ingress_format);
        }
    }
}
