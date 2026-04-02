// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::plonk::PlonkVersion;
use dusk_core::signatures::bls::BlsVersion;
use dusk_vm::{FeatureActivation, host_queries};
use node_data::hard_fork::{self, HardFork};

use super::{
    FEATURE_HARDFORK_AEGIS, FEATURE_HARDFORK_BOREAS, FEATURE_PLONK_V2,
    RuskVmConfig,
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
/// Fork-coupled execution policy resolved at a specific block height.
pub(super) struct ForkPolicy {
    pub(super) hard_fork: HardFork,
    pub(super) plonk_version: PlonkVersion,
    pub(super) bls_version: BlsVersion,
}

/// Applies VM-configured activations to the canonical hard-fork timeline.
///
/// This keeps feature-based config (`Height` / `Ranges`) and
/// `node_data::hard_fork` in sync.
pub(super) fn set_hard_fork_activations(vm_config: &RuskVmConfig) {
    hard_fork::set_aegis_activation_height(feature_activation_start(
        vm_config,
        FEATURE_HARDFORK_AEGIS,
    ));
    hard_fork::set_boreas_activation_height(feature_activation_start(
        vm_config,
        FEATURE_HARDFORK_BOREAS,
    ));
}

/// Resolves the execution policy used by Rusk at `block_height`.
///
/// Execution policy is the fork marker plus fork-dependent cryptographic
/// versions (Plonk and BLS) used for validation and VM host behavior.
pub(super) fn policy_at(
    vm_config: &RuskVmConfig,
    block_height: u64,
) -> ForkPolicy {
    let hard_fork = hard_fork::hard_fork_at(block_height);

    ForkPolicy {
        hard_fork,
        plonk_version: plonk_version_at(vm_config, block_height, hard_fork),
        bls_version: hard_fork.bls_version(),
    }
}

/// Maps chain hard-fork values to the VM host-query hard-fork enum.
///
/// This isolates the type boundary between `node_data` and `dusk_vm`.
pub(super) fn host_hard_fork(hard_fork: HardFork) -> host_queries::HardFork {
    match hard_fork {
        HardFork::PreFork => host_queries::HardFork::PreFork,
        HardFork::Aegis => host_queries::HardFork::Aegis,
        HardFork::Boreas => host_queries::HardFork::Boreas,
    }
}

fn feature_activation_start(vm_config: &RuskVmConfig, feature: &str) -> u64 {
    match vm_config.feature(feature) {
        Some(FeatureActivation::Height(height)) => *height,
        Some(FeatureActivation::Ranges(ranges)) => ranges
            .iter()
            .map(|(start, _)| *start)
            .min()
            .unwrap_or(u64::MAX),
        None => u64::MAX,
    }
}

fn plonk_version_at(
    vm_config: &RuskVmConfig,
    block_height: u64,
    hard_fork: HardFork,
) -> PlonkVersion {
    match hard_fork {
        HardFork::PreFork => {
            if vm_config.feature_active_at(FEATURE_PLONK_V2, block_height) {
                PlonkVersion::V2
            } else {
                PlonkVersion::V1
            }
        }
        HardFork::Aegis | HardFork::Boreas => PlonkVersion::V3,
    }
}

#[cfg(test)]
mod tests {
    use dusk_vm::FeatureActivation;

    use super::*;

    #[test]
    fn feature_activation_start_respects_ranges_and_heights() {
        let mut vm_config = RuskVmConfig::new();
        vm_config.with_feature(
            "test_ranges",
            FeatureActivation::Ranges(vec![(100, 150), (90, 95)]),
        );
        vm_config.with_feature("test_height", FeatureActivation::Height(7));

        assert_eq!(feature_activation_start(&vm_config, "test_ranges"), 90);
        assert_eq!(feature_activation_start(&vm_config, "test_height"), 7);
        assert_eq!(feature_activation_start(&vm_config, "missing"), u64::MAX);
    }

    #[test]
    fn plonk_version_tracks_prefork_and_aegis() {
        let mut vm_config = RuskVmConfig::new();
        vm_config
            .with_feature(FEATURE_PLONK_V2, FeatureActivation::Height(100));

        assert_eq!(
            plonk_version_at(&vm_config, 99, HardFork::PreFork),
            PlonkVersion::V1
        );
        assert_eq!(
            plonk_version_at(&vm_config, 100, HardFork::PreFork),
            PlonkVersion::V2
        );
        assert_eq!(
            plonk_version_at(&vm_config, 200, HardFork::Aegis),
            PlonkVersion::V3
        );
        assert_eq!(
            plonk_version_at(&vm_config, 300, HardFork::Boreas),
            PlonkVersion::V3
        );
    }
}
