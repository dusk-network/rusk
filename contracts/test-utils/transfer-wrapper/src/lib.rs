use dusk_abi::{ContractId, Transaction};
use dusk_bytes::Serializable;
use dusk_pki::{PublicSpendKey, SecretSpendKey, ViewKey};
use lazy_static::lazy_static;
use phoenix_core::{Crossover, Fee, Note};
use rand::{CryptoRng, RngCore};
use rusk_abi::RuskModule;
use rusk_vm::{Contract, GasMeter, NetworkState, VMError};
use transfer_circuits::ExecuteCircuit;
use transfer_contract::{Call, Error as TransferError, TransferContract};

use std::convert::{TryFrom, TryInto};

use dusk_plonk::prelude::*;

const TRANSFER: &'static [u8] = include_bytes!(
    "../../../../target/wasm32-unknown-unknown/release/transfer_contract.wasm"
);

lazy_static! {
    static ref PP: PublicParameters = unsafe {
        let pp = rusk_profile::get_common_reference_string().unwrap();

        PublicParameters::from_slice_unchecked(pp.as_slice())
    };
}

pub fn transfer_contract() -> ContractId {
    ContractId::from([
        0xd3, 0xf8, 0x7f, 0xfc, 0x1b, 0xc7, 0x43, 0x1d, 0xde, 0x81, 0x5f, 0xb1,
        0xe1, 0x1b, 0xd0, 0xfe, 0x88, 0x37, 0x1a, 0x15, 0x4a, 0xec, 0x27, 0x5d,
        0xed, 0x2, 0x4d, 0x8c, 0xc0, 0xf7, 0x99, 0x5f,
    ])
}

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
    let transfer = network
        .deploy(transfer)
        .map_err(|_| TransferError::ContractNotFound)?;

    assert_eq!(transfer, transfer_contract());

    Ok((network, ssk))
}

pub fn transfer_state(
    network: &NetworkState,
) -> Result<TransferContract, VMError> {
    network.get_contract_cast_state(&transfer_contract())
}

fn circuit_key(circuit_id: &[u8; 32]) -> Result<ProverKey, VMError> {
    let pk = rusk_profile::keys_for(circuit_id)
        .and_then(|keys| keys.get_prover())
        .map_err(|e| VMError::ContractPanic(e.to_string()))?;
    let pk = ProverKey::from_slice(pk.as_slice())
        .map_err(|e| VMError::ContractPanic(e.to_string()))?;

    Ok(pk)
}

fn prepare_execute<'a, R, I>(
    rng: &mut R,
    network: &NetworkState,
    inputs: I,
    output: &PublicSpendKey,
    transparent_output: bool,
    gas_limit: u64,
    gas_price: u64,
    gas_refund: &ViewKey,
    crossover: u64,
) -> Result<
    (
        GasMeter,
        BlsScalar,
        Vec<BlsScalar>,
        Fee,
        Crossover,
        Vec<Note>,
        Vec<u8>,
    ),
    VMError,
>
where
    R: RngCore + CryptoRng,
    I: Iterator<Item = &'a (SecretSpendKey, Note)>,
{
    let meter = GasMeter::with_limit(gas_limit);
    let anchor = transfer_state(network)?
        .notes()
        .inner()
        .root()
        .map_err(|e| VMError::ContractPanic(e.to_string()))?;

    let mut execute_proof = ExecuteCircuit::default();
    let mut input = 0;

    let nullifiers: Vec<BlsScalar> = inputs
        .map(|(ssk, note)| {
            let vk = ssk.view_key();
            let value = note
                .value(Some(&vk))
                .map_err(|e| VMError::ContractPanic(e.to_string()))?;

            input += value;

            let opening = transfer_state(network)?
                .notes()
                .opening(*note.pos())
                .map_err(|e| VMError::ContractPanic(e.to_string()))?
                .ok_or(VMError::ContractPanic(
                    "Tree opening failed".to_owned(),
                ))?;

            let signature = ExecuteCircuit::sign(rng, &ssk, note);
            execute_proof
                .add_input(&ssk, *note, opening, signature)
                .map_err(|e| VMError::ContractPanic(e.to_string()))?;

            Ok(note.gen_nullifier(ssk))
        })
        .collect::<Result<_, VMError>>()?;

    let mut outputs = vec![];
    let output_value = input - gas_limit - crossover;

    if output_value == 0 {
    } else if transparent_output {
        let note = Note::transparent(rng, output, output_value);
        let blinding_factor = note.blinding_factor(None).expect("Unreachable");

        execute_proof
            .add_output_with_data(note, output_value, blinding_factor)
            .map_err(|e| VMError::ContractPanic(e.to_string()))?;

        outputs.push(note);
    } else {
        let blinding_factor = JubJubScalar::random(rng);
        let note = Note::obfuscated(rng, output, output_value, blinding_factor);

        execute_proof
            .add_output_with_data(note, output_value, blinding_factor)
            .map_err(|e| VMError::ContractPanic(e.to_string()))?;

        outputs.push(note);
    }

    let gas_refund_psk = gas_refund.public_spend_key();
    let blinding_factor = JubJubScalar::random(rng);
    let note =
        Note::obfuscated(rng, &gas_refund_psk, crossover, blinding_factor);

    let (mut fee, crossover) =
        note.try_into().map_err(|e: phoenix_core::Error| {
            VMError::ContractPanic(e.to_string())
        })?;

    fee.gas_limit = gas_limit;
    fee.gas_price = gas_price;

    execute_proof
        .set_fee_crossover(&fee, &crossover, gas_refund)
        .map_err(|e| VMError::ContractPanic(e.to_string()))?;

    let id = execute_proof.circuit_id();
    let pk = circuit_key(id)?;

    let proof = execute_proof
        .gen_proof(&*PP, &pk, b"dusk-network")
        .map_err(|e| VMError::ContractPanic(e.to_string()))?
        .to_bytes()
        .to_vec();

    Ok((meter, anchor, nullifiers, fee, crossover, outputs, proof))
}

pub fn execute<'a, R, I>(
    rng: &mut R,
    network: &mut NetworkState,
    inputs: I,
    output: &PublicSpendKey,
    transparent_output: bool,
    gas_limit: u64,
    gas_price: u64,
    gas_refund: &SecretSpendKey,
    crossover: u64,
    call: Option<(ContractId, Transaction)>,
) -> Result<(), VMError>
where
    R: RngCore + CryptoRng,
    I: Iterator<Item = &'a (SecretSpendKey, Note)>,
{
    let gas_refund_vk = gas_refund.view_key();

    let (
        mut meter,
        anchor,
        nullifiers,
        fee,
        crossover_instance,
        outputs,
        spend_proof_execute,
    ) = prepare_execute(
        rng,
        network,
        inputs,
        output,
        transparent_output,
        gas_limit,
        gas_price,
        &gas_refund_vk,
        crossover,
    )?;

    let crossover = if crossover > 0 {
        Some(crossover_instance)
    } else {
        None
    };

    let call = Call::execute(
        anchor,
        nullifiers,
        fee,
        crossover,
        outputs,
        spend_proof_execute,
        call,
    );

    network.transact::<_, ()>(transfer_contract(), call, &mut meter)
}
