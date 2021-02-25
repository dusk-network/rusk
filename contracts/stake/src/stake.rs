// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod call;
mod counter;
mod key;
mod stake;

pub use call::Call;
use canonical::{Canon, Store};
use canonical_derive::Canon;
pub use counter::Counter;
use dusk_abi::ContractId;
use dusk_kelvin_map::Map;
pub use key::Key;
pub use stake::Stake;

/// The staking contract. It contains a mapping of a provisioner's public key to
/// his stake value and some extra info, as well as a set which contains all
/// provisioner public keys in order of being added to the contract. The
/// contract is responsible for maintaining the committee and allows users to
/// start staking, extend their stakes, and withdraw their stakes.
///
/// Note that rewards are distributed in a separate contract, this contract is
/// purely for management of stake lifetimes.
#[derive(Default, Debug, Clone, Canon)]
pub struct StakeContract<S: Store> {
    pub(crate) transfer_contract: ContractId,
    pub(crate) arbitration_contract: ContractId,
    pub(crate) stake_mapping: Map<Key, Stake, S>,
    pub(crate) stake_identifier_set: Map<Counter, Key, S>,
    pub(crate) counter: Counter,
}
