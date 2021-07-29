// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_abi::{ContractId, Transaction};
use dusk_bytes::Serializable;
use dusk_pki::{Ownable, PublicSpendKey, SecretSpendKey, ViewKey};
use lazy_static::lazy_static;
use phoenix_core::{Crossover, Fee, Note};
use rand::{CryptoRng, RngCore};
use rusk_abi::RuskModule;
use rusk_vm::{Contract, GasMeter, NetworkState, VMError};
use transfer_circuits::ExecuteCircuit;
use transfer_contract::{Call, Error as TransferError, TransferContract};

use std::convert::TryFrom;
use std::io;

use dusk_plonk::prelude::*;

static TRANSFER: &[u8] = include_bytes!(
    "../../../target/wasm32-unknown-unknown/release/transfer_contract.wasm"
);

lazy_static! {
    static ref PP: PublicParameters = unsafe {
        let pp = rusk_profile::get_common_reference_string().unwrap();

        PublicParameters::from_slice_unchecked(pp.as_slice())
    };
}

/// Create a new network state with the provided block height with an initial
/// unspent transparent note with the value specified in `balance`
pub fn genesis<R>(
    rng: &mut R,
    block_height: u64,
    balance: u64,
) -> Result<(NetworkState, SecretSpendKey), TransferError>
where
    R: RngCore + CryptoRng,
{
    let mut network = NetworkState::with_block_height(block_height);

    let ssk = SecretSpendKey::random(rng);
    let psk = PublicSpendKey::from(&ssk);

    let transfer = if balance > 0 {
        let genesis = Note::transparent(rng, &psk, balance);

        TransferContract::try_from(genesis)?
    } else {
        TransferContract::default()
    };

    let rusk_mod = RuskModule::new(&*PP);
    network.register_host_module(rusk_mod);

    let transfer = Contract::new(transfer, TRANSFER.to_vec());
    let transfer_id = rusk_abi::transfer_contract();
    let transfer = network
        .deploy_with_id(transfer_id, transfer)
        .or(Err(TransferError::ContractNotFound))?;

    assert_eq!(transfer, rusk_abi::transfer_contract());

    Ok((network, ssk))
}

/// Fetch an owned state of the transfer contract from a given network state
pub fn transfer_state(
    network: &NetworkState,
) -> Result<TransferContract, TransferError> {
    network
        .get_contract_cast_state(&rusk_abi::transfer_contract())
        .or(Err(TransferError::ContractNotFound))
}

/// Iterate all the notes, starting from the provided block height, and filter
/// the ones owned by the provided `ViewKey`
pub fn transfer_notes_owned_by(
    network: &NetworkState,
    block_height: u64,
    vk: &ViewKey,
) -> Result<Vec<Note>, TransferError> {
    let notes = transfer_state(network)?
        .notes_from_height(block_height)?
        .map(|n| n.map(|n| *n))
        .collect::<Result<Vec<Note>, TransferError>>()?
        .into_iter()
        .filter(|n| vk.owns(n.stealth_address()))
        .collect();

    Ok(notes)
}

/// Helper private function to return the ProverKey from a given circuit id
fn circuit_key(circuit_id: &[u8; 32]) -> Result<ProverKey, TransferError> {
    let pk = rusk_profile::keys_for(circuit_id)
        .and_then(|keys| keys.get_prover())
        .or(Err(TransferError::ProofVerificationError))?;

    let pk = ProverKey::from_slice(pk.as_slice())
        .or(Err(TransferError::ProofVerificationError))?;

    Ok(pk)
}

/// Helper function to generate a valid execute call with its ZK proof of
/// validity.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::type_complexity)]
fn prepare_execute<'a, R, I>(
    rng: &mut R,
    network: &NetworkState,
    inputs: I,
    output: &PublicSpendKey,
    transparent_output: bool,
    gas_refund: &ViewKey,
    fee: &Fee,
    crossover: Option<&Crossover>,
    crossover_value: u64,
) -> Result<
    (GasMeter, BlsScalar, Vec<BlsScalar>, Vec<Note>, Vec<u8>),
    TransferError,
>
where
    R: RngCore + CryptoRng,
    I: Iterator<Item = &'a (SecretSpendKey, Note)>,
{
    let meter = GasMeter::with_limit(fee.gas_limit);
    let anchor = transfer_state(network)?.notes().inner().root()?;

    let mut execute_proof = ExecuteCircuit::default();
    let mut input = 0;

    let nullifiers: Vec<BlsScalar> = inputs
        .map(|(ssk, note)| {
            let vk = ssk.view_key();
            let value = note.value(Some(&vk))?;

            input += value;

            let opening = transfer_state(network)?
                .notes()
                .opening(*note.pos())?
                .ok_or(TransferError::NoteNotFound)?;

            let signature = ExecuteCircuit::sign(rng, ssk, note);
            execute_proof
                .add_input(ssk, *note, opening, signature)
                .or(Err(TransferError::ProofVerificationError))?;

            Ok(note.gen_nullifier(ssk))
        })
        .collect::<Result<_, TransferError>>()?;

    let mut outputs = vec![];
    let output_value = input - fee.gas_limit - crossover_value;

    if output_value == 0 {
    } else if transparent_output {
        let note = Note::transparent(rng, output, output_value);
        let blinding_factor = note.blinding_factor(None).expect("Unreachable");

        execute_proof
            .add_output_with_data(note, output_value, blinding_factor)
            .or(Err(TransferError::ProofVerificationError))?;

        outputs.push(note);
    } else {
        let blinding_factor = JubJubScalar::random(rng);
        let note = Note::obfuscated(rng, output, output_value, blinding_factor);

        execute_proof
            .add_output_with_data(note, output_value, blinding_factor)
            .or(Err(TransferError::ProofVerificationError))?;

        outputs.push(note);
    }

    match crossover {
        Some(crossover) => {
            execute_proof.set_fee_crossover(fee, crossover, gas_refund)
        }
        None => execute_proof.set_fee(fee),
    }
    .or(Err(TransferError::ProofVerificationError))?;

    let id = execute_proof.circuit_id();
    let pk = circuit_key(id)?;

    let proof = execute_proof
        .gen_proof(&*PP, &pk, b"dusk-network")
        .or(Err(TransferError::ProofVerificationError))?
        .to_bytes()
        .to_vec();

    Ok((meter, anchor, nullifiers, outputs, proof))
}

/// Execute a transaction in the network.
///
/// Every transaction must go through the transfer execute method so the gas can
/// be properly handled.
///
/// The `inputs` is an iterator of a pair containing the secret spend key and
/// its note. After the execution, the provided notes will be spent.
///
/// An output note will be created with the difference between the spent value
/// and the inputs sum. This new note will be owned by the `output` key. This
/// note will be transparent if the `transparent_output` flag is true;
/// otherwise, it will be obfuscated.
///
/// The `fee` is the amount of gas reserved from the input that will be consumed
/// to execute the transaction. The unused gas will be refund to the key
/// `gas_refund` as a transparent note.
///
/// The `crossover` will be available in the context to be used with obfuscated
/// methods of the transfer contract. The `ViewKey` is used to open its value
/// commitment
///
/// The `call` is an encoded transaction to be executed in the network. If no
/// transaction is specified, no call is performed but the gas is still consumed
/// and the operation is valid.
#[allow(clippy::too_many_arguments)]
pub fn execute<'a, R, I>(
    rng: &mut R,
    network: &mut NetworkState,
    inputs: I,
    output: &PublicSpendKey,
    transparent_output: bool,
    gas_refund: &ViewKey,
    fee: Fee,
    crossover: Option<(&ViewKey, Crossover)>,
    call: Option<(ContractId, Transaction)>,
) -> Result<(), VMError>
where
    R: RngCore + CryptoRng,
    I: Iterator<Item = &'a (SecretSpendKey, Note)>,
{
    let crossover_value = crossover
        .map(|(vk, crossover)| {
            Note::from((fee, crossover))
                .value(Some(vk))
                .or(Err(VMError::InvalidArguments))
        })
        .transpose()?
        .unwrap_or(0);

    let crossover = crossover.map(|(_, crossover)| crossover);
    let (mut meter, anchor, nullifiers, outputs, spend_proof_execute) =
        prepare_execute(
            rng,
            network,
            inputs,
            output,
            transparent_output,
            gas_refund,
            &fee,
            crossover.as_ref(),
            crossover_value,
        )
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    let call = Call::execute(
        anchor,
        nullifiers,
        fee,
        crossover,
        outputs,
        spend_proof_execute,
        call,
    );

    network.transact::<_, ()>(rusk_abi::transfer_contract(), call, &mut meter)
}
