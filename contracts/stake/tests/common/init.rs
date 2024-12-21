// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::stake::STAKE_CONTRACT;
use dusk_core::transfer::{
    phoenix::{Note, PublicKey as PhoenixPublicKey},
    TRANSFER_CONTRACT,
};
use dusk_core::JubJubScalar;
use dusk_vm::{ContractData, Session, VM};
use ff::Field;
use rand::{CryptoRng, RngCore};

use crate::common::utils::{update_root, GAS_LIMIT};

const OWNER: [u8; 32] = [0; 32];
pub const CHAIN_ID: u8 = 0xFA;

/// Instantiate the virtual machine with the transfer contract deployed, with a
/// single note owned by the given public spend key.
pub fn instantiate<Rng: RngCore + CryptoRng>(
    rng: &mut Rng,
    vm: &VM,
    pk: &PhoenixPublicKey,
    genesis_value: u64,
) -> Session {
    let mut session = dusk_vm::new_genesis_session(vm, CHAIN_ID);

    // deploy transfer-contract
    let transfer_bytecode = include_bytes!(
        "../../../../target/dusk/wasm64-unknown-unknown/release/transfer_contract.wasm"
    );
    session
        .deploy(
            transfer_bytecode,
            ContractData::builder()
                .owner(OWNER)
                .contract_id(TRANSFER_CONTRACT),
            GAS_LIMIT,
        )
        .expect("Deploying the transfer contract should succeed");

    // deploy stake-contract
    let stake_bytecode = include_bytes!(
        "../../../../target/dusk/wasm32-unknown-unknown/release/stake_contract.wasm"
    );
    session
        .deploy(
            stake_bytecode,
            ContractData::builder()
                .owner(OWNER)
                .contract_id(STAKE_CONTRACT),
            GAS_LIMIT,
        )
        .expect("Deploying the stake contract should succeed");

    // create genesis-note
    let sender_blinder = [
        JubJubScalar::random(&mut *rng),
        JubJubScalar::random(&mut *rng),
    ];
    let genesis_note =
        Note::transparent(rng, pk, pk, genesis_value, sender_blinder);

    // push genesis note to the contract
    session
        .call::<_, Note>(
            TRANSFER_CONTRACT,
            "push_note",
            &(0u64, genesis_note),
            GAS_LIMIT,
        )
        .expect("Pushing genesis note should succeed");

    // update root of the tree of notes after genesis note has been pushed
    update_root(&mut session).expect("Updating the root should succeed");

    // sets the block height for all subsequent operations to 1
    let base = session.commit().expect("Committing should succeed");

    dusk_vm::new_session(vm, base, CHAIN_ID, 1)
        .expect("Instantiating new session should succeed")
}
