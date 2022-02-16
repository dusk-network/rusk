// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::*;

use dusk_abi::Transaction;
use dusk_bls12_381_sign::{PublicKey, Signature};
use phoenix_core::Note;
use transfer_contract::Call;

use alloc::vec::Vec;

#[cfg(not(feature = "no-bridge"))]
mod bridge;

impl StakeContract {
    pub fn stake(
        &mut self,
        pk: PublicKey,
        signature: Signature,
        value: u64,
        created_at: BlockHeight,
        proof: Vec<u8>,
    ) {
        if value < MINIMUM_STAKE {
            panic!("The staked value is not within range!");
        }

        let caller = dusk_abi::caller();
        if caller != rusk_abi::transfer_contract() {
            panic!("Can only be called from the transfer contract");
        }

        let block_height = dusk_abi::block_height();
        let stake = Stake::new(value, created_at, block_height);

        let message = Self::stake_sign_message(value, created_at);
        let signature = rusk_abi::verify_bls_sign(signature, pk, message);

        if !signature {
            panic!("The provided signature is invalid!");
        }

        let address = rusk_abi::stake_contract();
        let call = Call::send_to_contract_transparent(address, value, proof);
        let transaction = call.to_transaction();
        let transfer = rusk_abi::transfer_contract();

        dusk_abi::transact_raw(self, &transfer, &transaction, 0)
            .expect("Failed to send note to contract");

        self.push_stake(pk, stake, block_height).expect(
            "The provided key is already staked! It can only be extended",
        );
    }

    pub fn withdraw(
        &mut self,
        pk: PublicKey,
        signature: Signature,
        note: Note,
        withdraw_proof: Vec<u8>,
    ) {
        let stake = self.get_stake(&pk).expect("Failed to fetch stake");

        let message = Self::withdraw_sign_message(&stake, &note);
        let signature = rusk_abi::verify_bls_sign(signature, pk, message);

        if !signature {
            panic!("The provided signature is invalid!");
        }

        let value = stake.value();
        let call = Call::withdraw_from_transparent(value, note, withdraw_proof);

        let call = Transaction::from_canon(&call);
        let transfer = rusk_abi::transfer_contract();

        dusk_abi::transact_raw(self, &transfer, &call, 0)
            .expect("Failed to withdraw note from contract");

        self.remove_stake(&pk).expect("Failed to remove stake");
    }
}
