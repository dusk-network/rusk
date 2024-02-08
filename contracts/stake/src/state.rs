// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use core::cmp::min;

use crate::*;

use alloc::collections::BTreeMap;

use dusk_bls12_381_sign::PublicKey;
use dusk_bytes::Serializable;

use rusk_abi::{STAKE_CONTRACT, TRANSFER_CONTRACT};
use stake_contract_types::*;
use transfer_contract_types::*;

/// Contract keeping track of each public key's stake.
///
/// A caller can stake Dusk, and have it attached to a public key. This stake
/// has a maturation period, after which it is considered valid and the key
/// eligible to participate in the consensus.
///
/// Rewards may be received by a public key regardless of whether they have a
/// valid stake.
#[derive(Debug, Default, Clone)]
pub struct StakeState {
    stakes: BTreeMap<[u8; PublicKey::SIZE], StakeData>,
    slashed_amount: u64,
}

impl StakeState {
    pub const fn new() -> Self {
        Self {
            stakes: BTreeMap::new(),
            slashed_amount: 0u64,
        }
    }

    pub fn stake(&mut self, stake: Stake) {
        if stake.value < MINIMUM_STAKE {
            panic!("The staked value is lower than the minimum amount!");
        }

        // allot a stake to the given key and increment the signature counter
        let loaded_stake = self.load_or_create_stake_mut(&stake.public_key);

        let counter = loaded_stake.counter();

        loaded_stake.increment_counter();
        loaded_stake.insert_amount(stake.value, rusk_abi::block_height());

        // verify the signature is over the correct digest
        let digest = stake_signature_message(counter, stake.value).to_vec();

        if !rusk_abi::verify_bls(digest, stake.public_key, stake.signature) {
            panic!("Invalid signature!");
        }

        // make call to transfer contract to transfer balance from the user to
        // this contract
        let transfer_module = TRANSFER_CONTRACT;

        let stct = Stct {
            module: rusk_abi::self_id().to_bytes(),
            value: stake.value,
            proof: stake.proof,
        };

        let _: bool = rusk_abi::call(transfer_module, "stct", &stct)
            .expect("Sending note to contract should succeed");
    }

    pub fn unstake(&mut self, unstake: Unstake) {
        // remove the stake from a key and increment the signature counter
        let loaded_stake = self
            .get_stake_mut(&unstake.public_key)
            .expect("A stake should exist in the map to be unstaked!");

        let counter = loaded_stake.counter();

        let (value, _) = loaded_stake.remove_amount();
        loaded_stake.increment_counter();

        // verify signature
        let digest =
            unstake_signature_message(counter, unstake.note.as_slice());

        if !rusk_abi::verify_bls(digest, unstake.public_key, unstake.signature)
        {
            panic!("Invalid signature!");
        }
        // make call to transfer contract to withdraw a note from this contract
        // containing the value of the stake
        let transfer_module = TRANSFER_CONTRACT;
        let _: bool = rusk_abi::call(
            transfer_module,
            "wfct_raw",
            &WfctRaw {
                value,
                note: unstake.note,
                proof: unstake.proof,
            },
        )
        .expect("Withdrawing note from contract should be successful");
    }

    pub fn withdraw(&mut self, withdraw: Withdraw) {
        // deplete the stake from a key and increment the signature counter
        let loaded_stake = self
            .get_stake_mut(&withdraw.public_key)
            .expect("A stake should exist in the map to be withdrawn!");

        let counter = loaded_stake.counter();
        let reward = loaded_stake.reward();

        if reward == 0 {
            panic!("Nothing to withdraw!");
        }

        loaded_stake.deplete_reward();
        loaded_stake.increment_counter();

        // verify signature
        let digest = withdraw_signature_message(
            counter,
            withdraw.address,
            withdraw.nonce,
        )
        .to_vec();

        if !rusk_abi::verify_bls(
            digest,
            withdraw.public_key,
            withdraw.signature,
        ) {
            panic!("Invalid signature!");
        }

        // make call to transfer contract to mint the reward to the given
        // address
        let transfer_module = TRANSFER_CONTRACT;
        let _: bool = rusk_abi::call(
            transfer_module,
            "mint",
            &Mint {
                address: withdraw.address,
                value: reward,
                nonce: withdraw.nonce,
            },
        )
        .expect("Minting a reward note should succeed");
    }

    /// Gets a reference to a stake.
    pub fn get_stake(&self, key: &PublicKey) -> Option<&StakeData> {
        self.stakes.get(&key.to_bytes())
    }

    /// Gets a mutable reference to a stake.
    pub fn get_stake_mut(&mut self, key: &PublicKey) -> Option<&mut StakeData> {
        self.stakes.get_mut(&key.to_bytes())
    }

    /// Pushes the given `stake` onto the state for a given `public_key`.
    pub fn insert_stake(&mut self, public_key: PublicKey, stake: StakeData) {
        self.stakes.insert(public_key.to_bytes(), stake);
    }

    /// Gets a mutable reference to the stake of a given key. If said stake
    /// doesn't exist, a default one is inserted and a mutable reference
    /// returned.
    pub(crate) fn load_or_create_stake_mut(
        &mut self,
        pk: &PublicKey,
    ) -> &mut StakeData {
        let is_missing = self.stakes.get(&pk.to_bytes()).is_none();

        if is_missing {
            let stake = StakeData::default();
            self.stakes.insert(pk.to_bytes(), stake);
        }

        // SAFETY: unwrap is ok since we're sure we inserted an element
        self.stakes.get_mut(&pk.to_bytes()).unwrap()
    }

    /// Rewards a `public_key` with the given `value`. If a stake does not exist
    /// in the map for the key one will be created.
    pub fn reward(&mut self, public_key: &PublicKey, value: u64) {
        let stake = self.load_or_create_stake_mut(public_key);
        stake.increase_reward(value);
    }

    /// Total amount slashed from the genesis
    pub fn slashed_amount(&self) -> u64 {
        self.slashed_amount
    }

    /// Slash the given `to_subtract` amount from a `public_key` stake (if
    /// any). Firstly the amount is subtracted from the reward, if that's
    /// not enough the stake amount is touched.
    pub fn slash(&mut self, public_key: &PublicKey, to_slash: u64) {
        let stake = self
            .get_stake_mut(public_key)
            .expect("The stake to slash should exist");

        let (stake_amt, eligibility) = stake
            .amount
            .as_mut()
            .expect("The stake to slash should be active");

        if !stake.rewarded {
            *eligibility = next_epoch(rusk_abi::block_height());
            return;
        }

        let staker_funds = stake.reward + stake.amount.unwrap_or_default().0;

        // Cannot slash more than the staker funds
        let to_slash = min(to_slash, staker_funds);

        if to_slash <= stake.reward {
            stake.reward -= to_slash;
        } else {
            // Deplete reward and update `to_slash` with the remaining amount to
            // slash from the stake amount.
            let remaining_slash = to_slash - stake.reward;
            stake.reward = 0;

            *stake_amt -= remaining_slash;

            // Update the module balance to reflect the change in the amount
            // withdrawable from the contract
            let _: bool = rusk_abi::call(
                TRANSFER_CONTRACT,
                "sub_module_balance",
                &(STAKE_CONTRACT, remaining_slash),
            )
            .expect("Subtracting balance should succeed");
        }

        // Update the total slashed amount
        self.slashed_amount += to_slash;
    }

    /// Feeds the host with the stakes.
    pub fn stakes(&self) {
        for (k, v) in self.stakes.iter() {
            let pk = PublicKey::from_bytes(k).unwrap();
            let stake_data = v.clone();
            rusk_abi::feed((pk, stake_data));
        }
    }
}
