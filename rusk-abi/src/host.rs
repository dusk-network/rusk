// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::vec::Vec;
use std::path::{Path, PathBuf};

use blake2b_simd::Params;
use dusk_bytes::DeserializableSlice;
use dusk_poseidon::{Domain, Hash as PoseidonHash};
use execution_core::groth16::bn254::{Bn254, G1Projective};
use execution_core::groth16::serialize::CanonicalDeserialize;
use execution_core::groth16::{
    Groth16, PreparedVerifyingKey, Proof as Groth16Proof,
};
use execution_core::plonk::{Proof as PlonkProof, Verifier};
use execution_core::signatures::bls::{
    MultisigPublicKey, MultisigSignature, PublicKey as BlsPublicKey,
    Signature as BlsSignature,
};
use execution_core::signatures::schnorr::{
    PublicKey as SchnorrPublicKey, Signature as SchnorrSignature,
};
use execution_core::transfer::{Transaction, TRANSFER_CONTRACT};
use execution_core::{BlsScalar, ContractError};
use piecrust::{
    CallReceipt, ContractId, Error as PiecrustError, Session, SessionData,
    CONTRACT_ID_BYTES, VM,
};
use rkyv::ser::serializers::AllocSerializer;
use rkyv::{Archive, Deserialize, Serialize};

mod cache;
mod deploy;

use crate::{Metadata, Query};

/// Executes a transaction, returning the receipt of the call and the gas spent.
/// The following steps are performed:
///
/// 1. Check if the transaction contains contract deployment data, and if so,
///    verifies if gas limit is enough for deployment and if the gas price is
///    sufficient for deployment. If either gas price or gas limit is not
///    sufficient for deployment, transaction is discarded.
///
/// 2. Call the "spend_and_execute" function on the transfer contract with
///    unlimited gas. If this fails, an error is returned. If an error is
///    returned the transaction should be considered unspendable/invalid, but no
///    re-execution of previous transactions is required.
///
/// 3. If the transaction contains contract deployment data, additional checks
///    are performed and if they pass, deployment is executed. The following
///    checks are performed:
///    - gas limit should be is smaller than deploy charge plus gas used for
///      spending funds
///    - transaction's bytecode's bytes are consistent with bytecode's hash
///    Deployment execution may fail for deployment-specific reasons, such as
///    for example:
///    - contract already deployed
///    - corrupted bytecode
///    If deployment execution fails, the entire gas limit is consumed and error
///    is returned.
///
/// 4. Call the "refund" function on the transfer contract with unlimited gas.
///    The amount charged depends on the gas spent by the transaction, and the
///    optional contract call in steps 2 or 3.
///
/// Note that deployment transaction will never be re-executed for reasons
/// related to deployment, as it is either discarded or it charges the
/// full gas limit. It might be re-executed only if some other transaction
/// failed to fit the block.
pub fn execute(
    session: &mut Session,
    tx: &Transaction,
    gas_per_deploy_byte: u64,
    min_deployment_gas_price: u64,
) -> Result<CallReceipt<Result<Vec<u8>, ContractError>>, PiecrustError> {
    // Transaction will be discarded if it is a deployment transaction
    // with gas limit smaller than deploy charge.
    deploy::pre_check(tx, gas_per_deploy_byte, min_deployment_gas_price)?;

    // Spend the inputs and execute the call. If this errors the transaction is
    // unspendable.
    let mut receipt = session.call::<_, Result<Vec<u8>, ContractError>>(
        TRANSFER_CONTRACT,
        "spend_and_execute",
        // if it's a deploy tx, we need to strip the bytecode before passing it
        // to the transfer contract
        tx.strip_off_bytecode().as_ref().unwrap_or(tx),
        tx.gas_limit(),
    )?;

    // If this is a deployment transaction and the call to "spend_and_execute"
    // was successful, we can now deploy the contract.
    deploy::contract(session, tx, gas_per_deploy_byte, &mut receipt);

    // Ensure all gas is consumed if there's an error in the contract call
    if receipt.data.is_err() {
        receipt.gas_spent = receipt.gas_limit;
    }

    // Refund the appropriate amount to the transaction. This call is guaranteed
    // to never error. If it does, then a programming error has occurred. As
    // such, the call to `Result::expect` is warranted.
    let refund_receipt = session
        .call::<_, ()>(
            TRANSFER_CONTRACT,
            "refund",
            &receipt.gas_spent,
            u64::MAX,
        )
        .expect("Refunding must succeed");

    receipt.events.extend(refund_receipt.events);

    Ok(receipt)
}

/// Create a new session based on the given `vm`. The vm *must* have been
/// created using [`new_vm`] or [`new_ephemeral_vm`].
pub fn new_session(
    vm: &VM,
    base: [u8; 32],
    chain_id: u8,
    block_height: u64,
) -> Result<Session, PiecrustError> {
    vm.session(
        SessionData::builder()
            .base(base)
            .insert(Metadata::CHAIN_ID, chain_id)?
            .insert(Metadata::BLOCK_HEIGHT, block_height)?,
    )
}

/// Create a new genesis session based on the given `vm`. The vm *must* have
/// been created using [`new_vm`] or [`new_ephemeral_vm`].
pub fn new_genesis_session(vm: &VM, chain_id: u8) -> Session {
    vm.session(
        SessionData::builder()
            .insert(Metadata::CHAIN_ID, chain_id)
            .expect("Inserting chain ID in metadata should succeed")
            .insert(Metadata::BLOCK_HEIGHT, 0)
            .expect("Inserting block height in metadata should succeed"),
    )
    .expect("Creating a genesis session should always succeed")
}

/// Create a new [`VM`] compliant with Dusk's specification.
pub fn new_vm<P: AsRef<Path> + Into<PathBuf>>(
    root_dir: P,
) -> Result<VM, PiecrustError> {
    let mut vm = VM::new(root_dir)?;
    register_host_queries(&mut vm);
    Ok(vm)
}

/// Creates a new [`VM`] with a temporary directory.
pub fn new_ephemeral_vm() -> Result<VM, PiecrustError> {
    let mut vm = VM::ephemeral()?;
    register_host_queries(&mut vm);
    Ok(vm)
}

fn register_host_queries(vm: &mut VM) {
    vm.register_host_query(Query::HASH, host_hash);
    vm.register_host_query(Query::POSEIDON_HASH, host_poseidon_hash);
    vm.register_host_query(Query::VERIFY_PLONK, host_verify_plonk);
    vm.register_host_query(
        Query::VERIFY_GROTH16_BN254,
        host_verify_groth16_bn254,
    );
    vm.register_host_query(Query::VERIFY_SCHNORR, host_verify_schnorr);
    vm.register_host_query(Query::VERIFY_BLS, host_verify_bls);
    vm.register_host_query(
        Query::VERIFY_BLS_MULTISIG,
        host_verify_bls_multisig,
    );
}

fn wrap_host_query<A, R, F>(arg_buf: &mut [u8], arg_len: u32, closure: F) -> u32
where
    F: FnOnce(A) -> R,
    A: Archive,
    A::Archived: Deserialize<A, rkyv::Infallible>,
    R: Serialize<AllocSerializer<1024>>,
{
    let root =
        unsafe { rkyv::archived_root::<A>(&arg_buf[..arg_len as usize]) };
    let arg: A = root.deserialize(&mut rkyv::Infallible).unwrap();

    let result = closure(arg);

    let bytes = rkyv::to_bytes::<_, 1024>(&result).unwrap();

    arg_buf[..bytes.len()].copy_from_slice(&bytes);
    bytes.len() as u32
}

fn host_hash(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    wrap_host_query(arg_buf, arg_len, hash)
}

fn host_poseidon_hash(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    wrap_host_query(arg_buf, arg_len, poseidon_hash)
}

fn host_verify_plonk(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    let hash = *blake2b_simd::blake2b(&arg_buf[..arg_len as usize]).as_array();
    let cached = cache::get_plonk_verification(hash);

    wrap_host_query(arg_buf, arg_len, |(vd, proof, pis)| {
        let is_valid = cached.unwrap_or_else(|| verify_plonk(vd, proof, pis));
        cache::put_plonk_verification(hash, is_valid);
        is_valid
    })
}

fn host_verify_groth16_bn254(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    let hash = *blake2b_simd::blake2b(&arg_buf[..arg_len as usize]).as_array();
    let cached = cache::get_groth16_verification(hash);

    wrap_host_query(arg_buf, arg_len, |(pvk, proof, inputs)| {
        let is_valid =
            cached.unwrap_or_else(|| verify_groth16_bn254(pvk, proof, inputs));
        cache::put_groth16_verification(hash, is_valid);
        is_valid
    })
}

fn host_verify_schnorr(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    wrap_host_query(arg_buf, arg_len, |(msg, pk, sig)| {
        verify_schnorr(msg, pk, sig)
    })
}

fn host_verify_bls(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    let hash = *blake2b_simd::blake2b(&arg_buf[..arg_len as usize]).as_array();
    let cached = cache::get_bls_verification(hash);

    wrap_host_query(arg_buf, arg_len, |(msg, pk, sig)| {
        let is_valid = cached.unwrap_or_else(|| verify_bls(msg, pk, sig));
        cache::put_bls_verification(hash, is_valid);
        is_valid
    })
}

fn host_verify_bls_multisig(arg_buf: &mut [u8], arg_len: u32) -> u32 {
    wrap_host_query(arg_buf, arg_len, |(msg, keys, sig)| {
        verify_bls_multisig(msg, keys, sig)
    })
}

/// Compute the blake2b hash of the given scalars, returning the resulting
/// scalar. The hash is computed in such a way that it will always return a
/// valid scalar.
pub fn hash(bytes: Vec<u8>) -> BlsScalar {
    BlsScalar::hash_to_scalar(&bytes[..])
}

/// Compute the poseidon hash of the given scalars
pub fn poseidon_hash(scalars: Vec<BlsScalar>) -> BlsScalar {
    PoseidonHash::digest(Domain::Other, &scalars)[0]
}

/// Verify a Plonk proof is valid for a given circuit type and public inputs
///
/// # Panics
/// This will panic if `verifier_data` or `proof` are not valid.
pub fn verify_plonk(
    verifier_data: Vec<u8>,
    proof: Vec<u8>,
    public_inputs: Vec<BlsScalar>,
) -> bool {
    let verifier = Verifier::try_from_bytes(verifier_data)
        .expect("Verifier data coming from the contract should be valid");
    let proof = PlonkProof::from_slice(&proof).expect("Proof should be valid");

    verifier.verify(&proof, &public_inputs[..]).is_ok()
}

/// Verify that a Groth16 proof in the BN254 pairing is valid for a given
/// circuit and inputs.
///
/// `proof` and `inputs` should be in compressed form, while `pvk` uncompressed.
///
/// # Panics
/// This will panic if `pvk`, `proof` or `inputs` are not valid.
pub fn verify_groth16_bn254(
    pvk: Vec<u8>,
    proof: Vec<u8>,
    inputs: Vec<u8>,
) -> bool {
    let pvk = PreparedVerifyingKey::deserialize_uncompressed(&pvk[..])
        .expect("verifying key must be valid");
    let proof = Groth16Proof::deserialize_compressed(&proof[..])
        .expect("proof must be valid");
    let inputs = G1Projective::deserialize_compressed(&inputs[..])
        .expect("inputs must be valid");

    Groth16::<Bn254>::verify_proof_with_prepared_inputs(&pvk, &proof, &inputs)
        .expect("verifying proof should succeed")
}

/// Verify a schnorr signature is valid for the given public key and message
pub fn verify_schnorr(
    msg: BlsScalar,
    pk: SchnorrPublicKey,
    sig: SchnorrSignature,
) -> bool {
    pk.verify(&sig, msg).is_ok()
}

/// Verify a BLS signature is valid for the given public key and message
pub fn verify_bls(msg: Vec<u8>, pk: BlsPublicKey, sig: BlsSignature) -> bool {
    pk.verify(&sig, &msg).is_ok()
}

/// Verify a BLS signature is valid for the given public key and message
pub fn verify_bls_multisig(
    msg: Vec<u8>,
    keys: Vec<BlsPublicKey>,
    sig: MultisigSignature,
) -> bool {
    let len = keys.len();
    if len < 1 {
        panic!("must have at least one key");
    }

    let akey = MultisigPublicKey::aggregate(&keys)
        .expect("aggregation should succeed");

    akey.verify(&sig, &msg).is_ok()
}

/// Generate a [`ContractId`] address from:
/// - slice of bytes,
/// - nonce
/// - owner
pub fn gen_contract_id(
    bytes: impl AsRef<[u8]>,
    nonce: u64,
    owner: impl AsRef<[u8]>,
) -> ContractId {
    let mut hasher = Params::new().hash_length(CONTRACT_ID_BYTES).to_state();
    hasher.update(bytes.as_ref());
    hasher.update(&nonce.to_le_bytes()[..]);
    hasher.update(owner.as_ref());
    let hash_bytes: [u8; CONTRACT_ID_BYTES] = hasher
        .finalize()
        .as_bytes()
        .try_into()
        .expect("the hash result is exactly `CONTRACT_ID_BYTES` long");
    ContractId::from_bytes(hash_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::{RngCore, SeedableRng};

    #[test]
    fn test_gen_contract_id() {
        let mut rng = StdRng::seed_from_u64(42);

        let mut bytes = vec![0; 1000];
        rng.fill_bytes(&mut bytes);

        let nonce = rng.next_u64();

        let mut owner = vec![0, 100];
        rng.fill_bytes(&mut owner);

        let contract_id =
            gen_contract_id(bytes.as_slice(), nonce, owner.as_slice());

        assert_eq!(
            contract_id.as_bytes(),
            [
                45, 168, 182, 39, 119, 137, 168, 140, 114, 21, 120, 158, 34,
                126, 244, 221, 151, 72, 109, 178, 82, 229, 84, 128, 92, 123,
                135, 74, 23, 224, 119, 133
            ]
        );
    }
}
