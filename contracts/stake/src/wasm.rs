// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::*;

use alloc::vec::Vec;

use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::{APK, PublicKey, Signature};
use dusk_pki::StealthAddress;
use phoenix_core::Note;
use rusk_abi::State;
use rusk_abi::ModuleId;
use rusk_abi::RawTransaction;

use crate::wasm::StakeContract;
use rkyv::{Archive, Deserialize, Serialize};
use bytecheck::CheckBytes;


// use transfer_contract::Call;

// #[cfg(not(feature = "no-bridge"))]
// mod bridge;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub enum Call1 {
    // Execute {
    //     anchor: BlsScalar,
    //     nullifiers: Vec<BlsScalar>,
    //     fee: Fee,
    //     crossover: Option<Crossover>,
    //     notes: Vec<Note>,
    //     spend_proof: Vec<u8>,
    //     call: Option<(ContractId, Transaction)>,
    // },

    SendToContractTransparent {
        address: ModuleId,
        value: u64,
        spend_proof: Vec<u8>,
    },

    WithdrawFromTransparent {
        value: u64,
        note: Note,
        spend_proof: Vec<u8>,
    },
    //
    // SendToContractObfuscated {
    //     address: ContractId,
    //     message: Message,
    //     message_address: StealthAddress,
    //     spend_proof: Vec<u8>,
    // },
    //
    // WithdrawFromObfuscated {
    //     message: Message,
    //     message_address: StealthAddress,
    //     change: Message,
    //     change_address: StealthAddress,
    //     output: Note,
    //     spend_proof: Vec<u8>,
    // },
    //
    // WithdrawFromTransparentToContract {
    //     to: ContractId,
    //     value: u64,
    // },
    //
    Mint {
        address: StealthAddress,
        value: u64,
        nonce: BlsScalar,
    },
}


impl StakeContract {
    pub fn stake(
        self: &mut State<Self>,
        pk: PublicKey,
        signature: Signature,
        value: u64,
        proof: Vec<u8>,
    ) {
        if rusk_abi::caller() != rusk_abi::transfer_module() {
            panic!("Can only be called from the transfer contract!");
        }

        if !self.is_allowlisted(&pk) {
            panic!("The address is not allowed!");
        }

        if value < MINIMUM_STAKE {
            panic!("The staked value is lower than the minimum amount!");
        }

        // allot a stake to the given key and increment the signature counter
        let mut stake =
            self.load_mut_stake(&pk).expect("Failed to query state!");

        let counter = stake.counter();

        stake.increment_counter();
        stake.insert_amount(value, rusk_abi::block_height());

        // required since we're holding a mutable reference to a stake and
        // `dusk_abi::transact_raw` requires a mutable reference to the state
        drop(stake);

        // verify the signature is over the correct message
        let message = Self::stake_sign_message(counter, value);
        let valid_signature = rusk_abi::verify_bls(message, APK::from(&pk), signature );

        if !valid_signature {
            panic!("Invalid signature!");
        }

        // make call to transfer contract to transfer balance from the user to
        // this contract
        let transfer = rusk_abi::transfer_module();
        let transaction = {
            let address = rusk_abi::stake_module();
            let call =
                Call1::SendToContractTransparent{address, value, spend_proof: proof};
                // Stct{module: address, value, proof: Proof::from_bytes(proof.as_slice())?};
            //call.to_transaction()
            RawTransaction::new("stake", call)
        };

        self.transact_raw(transfer, transaction)
            .expect("Failed to send note to contract");
    }

    pub fn unstake(
        self: &mut State<Self>,
        pk: PublicKey,
        signature: Signature,
        note: Note,
        withdraw_proof: Vec<u8>,
    ) {
        if rusk_abi::caller() != rusk_abi::transfer_module() {
            panic!("Can only be called from the transfer contract!");
        }

        // remove the stake from a key and increment the signature counter
        let mut stake = self
            .get_stake_mut(&pk)
            .expect("The provided key has no stake!");

        let counter = stake.counter();

        let (value, _) = stake.remove_amount();
        stake.increment_counter();

        // required since we're holding a mutable reference to a stake and
        // `dusk_abi::transact_raw` requires a mutable reference to the state
        drop(stake);

        // verify signature
        let message = Self::unstake_sign_message(counter, note);
        let valid_signature = rusk_abi::verify_bls(message, APK::from(&pk), signature);

        if !valid_signature {
            panic!("Invalid signature!");
        }

        // make call to transfer contract to withdraw a note from this contract
        // containing the value of the stake
        let transfer = rusk_abi::transfer_module();
        // let transaction =
        //     Call::withdraw_from_transparent(value, note, withdraw_proof)
            // Wfct {value, note, proof: withdraw_proof}
            //     .to_transaction();
        let transaction = {
            let call =
                Call1::WithdrawFromTransparent{value, note, spend_proof: withdraw_proof};
            RawTransaction::new("unstake", call)
        };

        self.transact_raw(transfer, transaction)
            .expect("Failed to withdraw note from contract");
    }

    pub fn withdraw(
        self: &mut State<Self>,
        pk: PublicKey,
        signature: Signature,
        address: StealthAddress,
        nonce: BlsScalar,
    ) {
        if rusk_abi::caller() != rusk_abi::transfer_module() {
            panic!("Can only be called from the transfer contract!");
        }

        // deplete the stake from a key and increment the signature counter
        let mut stake = self
            .get_stake_mut(&pk)
            .expect("The provided key has no stake!");

        let counter = stake.counter();
        let reward = stake.reward();

        if reward == 0 {
            panic!("Nothing to withdraw!");
        }

        stake.deplete_reward();
        stake.increment_counter();

        // required since we're holding a mutable reference to a stake and
        // `dusk_abi::transact_raw` requires a mutable reference to the state
        drop(stake);

        // verify signature
        let message = Self::withdraw_sign_message(counter, address, nonce);
        let valid_signature = rusk_abi::verify_bls(message, APK::from(&pk), signature);

        if !valid_signature {
            panic!("Invalid signature!");
        }

        // make call to transfer contract to mint the reward to the given
        // address
        let transfer = rusk_abi::transfer_module();
        // let transaction = Mint {address, value: reward, nonce}.to_transaction();
        let transaction = {
            let call =
                Call1::Mint{address, value: reward, nonce};
            RawTransaction::new("withdraw", call)
        };

        self.transact_raw(transfer, transaction)
            .expect("Failed to mint reward note");
    }

    pub fn allowlist(
        &mut self,
        pk: PublicKey,
        signature: Signature,
        owner: PublicKey,
    ) {
        if rusk_abi::caller() != rusk_abi::transfer_module() {
            panic!("Can only be called from the transfer contract!");
        }

        if self.is_allowlisted(&pk) {
            panic!("Address already allowed!");
        }

        if !self.is_owner(&owner) {
            panic!("Can only be called by a contract owner!");
        }

        // deplete the stake from a key and increment the signature counter
        let mut owner_stake = self
            .get_stake_mut(&owner)
            .expect("The provided owner has no stake!");

        let owner_counter = owner_stake.counter();

        owner_stake.increment_counter();

        drop(owner_stake);
        // verify signature
        let message = Self::allowlist_sign_message(owner_counter, &pk);
        let valid_signature =
            rusk_abi::verify_bls(message, APK::from(&owner), signature);

        if !valid_signature {
            panic!("Invalid signature!");
        }

        self.insert_allowlist(pk);
    }
}
