// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use canonical::Canon;
use canonical_derive::Canon;
use dusk_bls12_381_sign::APK;

/// Stake represents a stake transaction performed in the Dusk network, and
/// contains info on it's size, sender, eligibility time, and expiration time.
#[derive(Debug, Default, Clone, Copy, Canon)]
pub struct Stake {
    /// The amount of DUSK staked by the provisioner.
    pub value: u64,
    /// The provisioner's public key.
    pub pk: APK,
    /// The block height at which this stake becomes active, allowing the
    /// provisioner to participate in SBA.
    pub eligibility: u64,
    /// The block height at which this stake becomes inactive.
    pub expiration: u64,
}
