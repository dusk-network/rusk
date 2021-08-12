// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bid_circuits::BidCorrectnessCircuit;
use bid_contract::Contract as BidContract;
use dusk_abi::{ContractId, Transaction};
use dusk_blindbid::{V_RAW_MAX, V_RAW_MIN};
use dusk_bytes::Serializable;
use dusk_jubjub::{
    JubJubAffine, JubJubScalar, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_pki::{PublicSpendKey, SecretSpendKey};
use dusk_plonk::prelude::*;
use dusk_poseidon::sponge;
use lazy_static::lazy_static;
use phoenix_core::{Crossover, Fee, Message, Note};
use rand::{CryptoRng, Rng, RngCore};
use rusk_vm::Contract;
use std::convert::TryInto;
use transfer_circuits::SendToContractObfuscatedCircuit;
use transfer_contract::TransferContract;

lazy_static! {
    static ref PP: PublicParameters = unsafe {
        let pp = rusk_profile::get_common_reference_string().unwrap();

        PublicParameters::from_slice_unchecked(pp.as_slice())
    };
}

const BID_CONTRACT_BYTECODE: &[u8] = include_bytes!(
    "../../../target/wasm32-unknown-unknown/release/bid_contract.wasm"
);

fn prove_bid(value: JubJubScalar, blinder: JubJubScalar) -> Proof {
    let commitment = JubJubAffine::from(
        (GENERATOR_EXTENDED * value) + (GENERATOR_NUMS_EXTENDED * blinder),
    );

    let mut circuit = BidCorrectnessCircuit {
        commitment,
        value,
        blinder,
    };

    let pk = rusk_profile::keys_for(&BidCorrectnessCircuit::CIRCUIT_ID)
        .expect("Failed to fetch circuit keys")
        .get_prover()
        .expect("Failed to get proverkey data");
    let pk = ProverKey::from_slice(&pk)
        .expect("Failed to deserialize the ProverKey");
    circuit.gen_proof(&PP, &pk, b"dusk-network").unwrap()
}

fn prove_stco<R: RngCore + CryptoRng>(
    rng: &mut R,
    ssk: &SecretSpendKey,
    fee: Fee,
    crossover: Crossover,
    message: Message,
    message_r: JubJubScalar,
    contract: &ContractId,
) -> Proof {
    let contract = TransferContract::contract_to_scalar(contract);
    let vk = ssk.view_key();
    let psk = PublicSpendKey::from(ssk);

    let signature = SendToContractObfuscatedCircuit::sign(
        rng, ssk, &fee, &crossover, &message, &contract,
    );

    let mut circuit = SendToContractObfuscatedCircuit::new(
        fee, crossover, &vk, signature, false, message, &psk, message_r,
        contract,
    )
    .expect("Error on circuit generation");

    let id = SendToContractObfuscatedCircuit::CIRCUIT_ID;
    let keys = rusk_profile::keys_for(&id)
        .expect("Failed to fetch keys from rusk-profile");
    let pk = keys
        .get_prover()
        .expect("Failed to extract prover key from rusk-profile");
    let pk = ProverKey::from_slice(pk.as_slice())
        .expect("Failed to parse prover key from rusk-profile");

    circuit
        .gen_proof(&*PP, &pk, b"dusk-network")
        .expect("Failed to generate proof")
}

#[test]
fn bid_contract_workflow_works() {
    let rng = &mut rand::thread_rng();

    // Init Env & Contract
    let contract =
        Contract::new(BidContract::new(), BID_CONTRACT_BYTECODE.to_vec());

    // Create BidCorrectnessCircuit Proof and send it
    let r = JubJubScalar::random(rng);
    let ssk = SecretSpendKey::random(rng);
    let vk = ssk.view_key();
    let psk = PublicSpendKey::from(&ssk);
    let stealth = psk.gen_stealth_address(&r);

    let secret = JubJubScalar::random(rng);
    let secret = sponge::hash(&[secret.into()]);

    let value: u64 = rng.gen_range(V_RAW_MIN..V_RAW_MAX);
    let message = Message::new(rng, &r, &psk, value);
    let (_, blinder) = message.decrypt(&r, &psk).expect("decryption error");

    let proof = prove_bid(value.into(), blinder).to_bytes().to_vec();

    // Generate env
    let block_height = 0u64;
    let (mut network, genesis_ssk) =
        transfer_wrapper::genesis(rng, block_height, 10_000_000_000_000)
            .expect("Failed to initialize a genesis state");

    let genesis_note = transfer_wrapper::transfer_notes_owned_by(
        &network,
        block_height,
        &genesis_ssk.view_key(),
    )
    .expect("Error fetching genesis note")[0];

    // Deploy Bid contract
    let bid_contract = network.deploy(contract).expect("Deploy failure");

    // Transfer Dusk to the Bid Contract (STCO)
    let note = Note::obfuscated(rng, &psk, value, blinder);
    let (mut fee, crossover) = note
        .try_into()
        .expect("Error on generating Fee and Crossover");

    fee.gas_limit = 1_000_000_000_000;
    fee.gas_price = 1;

    let stco_proof =
        prove_stco(rng, &ssk, fee, crossover, message, r, &bid_contract)
            .to_bytes()
            .to_vec();

    let tx = Transaction::from_canon(&(
        bid_contract::ops::BID,
        message,
        secret,
        stealth,
        proof,
        stco_proof,
    ));

    transfer_wrapper::execute(
        rng,
        &mut network,
        [(genesis_ssk, genesis_note)].iter(),
        &psk,
        true,
        &vk,
        fee,
        Some((&vk, crossover)),
        Some((bid_contract, tx)),
    )
    .expect("Failed execute tx build");
}
