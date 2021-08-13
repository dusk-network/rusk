// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bid_circuits::BidCorrectnessCircuit;
use bid_contract::{
    contract_constants::{MATURITY_PERIOD, VALIDITY_PERIOD},
    Contract as BidContract,
};
use dusk_abi::{ContractId, Transaction};
use dusk_blindbid::{V_RAW_MAX, V_RAW_MIN};
use dusk_bytes::Serializable;
use dusk_jubjub::{
    JubJubAffine, JubJubScalar, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_pki::{PublicKey, PublicSpendKey, SecretSpendKey, ViewKey};
use dusk_plonk::prelude::*;
use dusk_poseidon::sponge;
use dusk_schnorr::Signature;
use lazy_static::lazy_static;
use microkelvin::DiskBackend;
use phoenix_core::{Crossover, Fee, Message, Note};
use rand::{CryptoRng, Rng, RngCore};
use rusk_abi::RuskModule;
use rusk_vm::{Contract, NetworkState};
use std::convert::TryInto;
use transfer_circuits::{
    SendToContractObfuscatedCircuit, WithdrawFromObfuscatedCircuit,
};
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

fn prove_wfo(
    message_r: JubJubScalar,
    message_ssk: &SecretSpendKey,
    message: &Message,
    output: &Note,
    output_vk: Option<&ViewKey>,
) -> Proof {
    let mut circuit = WithdrawFromObfuscatedCircuit::new(
        message_r,
        message_ssk,
        message,
        output,
        output_vk,
    )
    .expect("Failed to generate circuit!");

    let id = WithdrawFromObfuscatedCircuit::CIRCUIT_ID;
    let keys = rusk_profile::keys_for(&id)
        .expect("Failed to fetch keys from rusk-profile");
    let pk = keys
        .get_prover()
        .expect("Failed to extract prover key from rusk-profile");
    let pk = ProverKey::from_slice(pk.as_slice())
        .expect("Failed to parse prover key from rusk-profile");

    let vd = keys
        .get_verifier()
        .expect("Failed to extract prover key from rusk-profile");
    let vd = VerifierData::from_slice(vd.as_slice())
        .expect("Failed to parse prover key from rusk-profile");

    let proof = circuit
        .gen_proof(&*PP, &pk, b"dusk-network")
        .expect("Failed to generate proof");

    assert!(dusk_plonk::circuit::verify_proof(
        &*PP,
        vd.key(),
        &proof,
        &circuit.public_inputs(),
        vd.pi_pos(),
        b"dusk-network",
    )
    .is_ok());

    proof
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

    // Create signature PublicKey
    let sig_key = PublicKey::from(&ssk.sk_r(&stealth));
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

    // The Bid is now placed inside of the Bid Tree of the contract.
    // If we try to extend the Bid with an un-increased block height, the op should fail.

    // Sign the t_e (expiration) of the Bid.
    let bad_signature = Signature::new(
        &ssk.sk_r(&stealth),
        &mut rand::thread_rng(),
        BlsScalar::from(block_height + MATURITY_PERIOD + VALIDITY_PERIOD),
    );

    let extend_fail_tx = Transaction::from_canon(&(
        bid_contract::ops::EXTEND_BID,
        bad_signature,
        sig_key,
    ));

    // Get notes owned by genesis_ssk
    let note = transfer_wrapper::transfer_notes_owned_by(
        &network,
        block_height,
        &genesis_ssk.view_key(),
    )
    .expect("Error fetching genesis note")
    .last()
    .unwrap()
    .clone();

    assert!(transfer_wrapper::execute(
        rng,
        &mut network,
        [(genesis_ssk, note)].iter(),
        &psk,
        true,
        &vk,
        fee,
        Some((&vk, crossover)),
        Some((bid_contract, extend_fail_tx)),
    )
    .is_err());

    // Update the block_height of the NetworkState and try to extend the Bid again.
    let persist_id = network
        .persist(|| {
            let dir = std::env::temp_dir().join("bid_contract_workflow_works");
            std::fs::create_dir_all(&dir).expect("Error on tmp dir creation");
            DiskBackend::new(dir)
        })
        .expect("Error in persistence");

    let block_height = VALIDITY_PERIOD + 1;
    // Generate new NetworkState with blockheight = MATURITY_PERIOD + 1
    let mut network = NetworkState::with_block_height(block_height)
        .restore(persist_id)
        .expect("Error reconstructing the NetworkState");
    let rusk_mod = RuskModule::new(&*PP);
    network.register_host_module(rusk_mod);

    // Sign the t_e (expiration) of the Bid.
    let signature = Signature::new(
        &ssk.sk_r(&stealth),
        &mut rand::thread_rng(),
        BlsScalar::from(MATURITY_PERIOD + VALIDITY_PERIOD),
    );

    let extend_tx = Transaction::from_canon(&(
        bid_contract::ops::EXTEND_BID,
        signature,
        sig_key,
    ));

    // Get notes owned by genesis_ssk
    let note = transfer_wrapper::transfer_notes_owned_by(&network, 0, &vk)
        .expect("Error fetching genesis note")
        .last()
        .unwrap()
        .clone();

    fee.gas_limit = note.value(Some(&vk)).unwrap() - 1000;
    transfer_wrapper::execute(
        rng,
        &mut network,
        [(ssk, note)].iter(),
        &psk,
        true,
        &vk,
        fee,
        None,
        Some((bid_contract, extend_tx)),
    )
    .expect("Failed to extend the bid");

    // Remove the persist data to generate a new one updated to the actual state.
    std::fs::remove_dir_all(
        std::env::temp_dir().join("bid_contract_workflow_works"),
    )
    .expect("teardown fn error");

    let persist_id = network
        .persist(|| {
            let dir = std::env::temp_dir().join("bid_contract_workflow_works");
            std::fs::create_dir_all(&dir).expect("Error on tmp dir creation");
            DiskBackend::new(dir)
        })
        .expect("Error in persistence");

    // Set a new NetworkState with a block_height that leaves the bid expired so that we can withrdraw it
    let block_height = MATURITY_PERIOD + 2 * VALIDITY_PERIOD + 1;
    // Generate new NetworkState with blockheight = MATURITY_PERIOD + 1
    let mut network = NetworkState::with_block_height(block_height)
        .restore(persist_id)
        .expect("Error reconstructing the NetworkState");
    let rusk_mod = RuskModule::new(&*PP);
    network.register_host_module(rusk_mod);

    // WITHDRAWAL OF THE BID
    // Sign the elegibility and call withdraw bid.
    let withdraw_signature = Signature::new(
        &ssk.sk_r(&stealth),
        &mut rand::thread_rng(),
        BlsScalar::from(MATURITY_PERIOD + 2 * VALIDITY_PERIOD),
    );

    // Get notes owned by genesis_ssk
    let note = transfer_wrapper::transfer_notes_owned_by(
        &network,
        VALIDITY_PERIOD + 1, // The block height at which the refund of the EXTEND_BID call was appended to the Note tree.
        &vk,
    )
    .expect("Error fetching genesis note")
    .last()
    .unwrap()
    .clone();

    fee.gas_limit = note.value(Some(&vk)).unwrap() - 1000;
    fee.gas_price = 1;

    let zero_value_note =
        Note::obfuscated(&mut rand::thread_rng(), &psk, 0, JubJubScalar::one());
    // Create new Id for the withdrawal
    let withdraw_ssk = SecretSpendKey::random(&mut rand::thread_rng());
    let withdraw_psk = withdraw_ssk.public_spend_key();
    let withdraw_vk = withdraw_ssk.view_key();
    let withdraw_note = Note::obfuscated(
        &mut rand::thread_rng(),
        &withdraw_psk,
        value,
        JubJubScalar::random(&mut rand::thread_rng()),
    );

    let wfo_proof =
        prove_wfo(r, &ssk, &message, &withdraw_note, Some(&withdraw_vk));

    let withdraw_tx = Transaction::from_canon(&(
        bid_contract::ops::WITHDRAW,
        withdraw_signature,
        sig_key,
        withdraw_note,
        wfo_proof.to_bytes().to_vec(),
    ));

    transfer_wrapper::execute(
        rng,
        &mut network,
        [(ssk, note)].iter(),
        &psk,
        true,
        &vk,
        fee,
        None,
        Some((bid_contract, withdraw_tx)),
    )
    .expect("Failed to withdraw the Bid");

    // Get notes owned by genesis_ssk
    let note = transfer_wrapper::transfer_notes_owned_by(
        &network,
        block_height,
        &withdraw_vk,
    )
    .expect("Error fetching withdraw note")
    .last()
    .unwrap()
    .clone();

    assert_eq!(note.value(Some(&withdraw_vk)).unwrap(), value);

    // Teardown
    std::fs::remove_dir_all(
        std::env::temp_dir().join("bid_contract_workflow_works"),
    )
    .expect("teardown fn error");
}
