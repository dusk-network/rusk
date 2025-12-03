// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod config;
pub mod feature;

use blake2b_simd::Params;
use dusk_core::abi::{ContractError, ContractId, Metadata, CONTRACT_ID_BYTES};
use dusk_core::stake::STAKE_CONTRACT;
use dusk_core::transfer::data::ContractBytecode;
use dusk_core::transfer::{Transaction, TRANSFER_CONTRACT};
use piecrust::{CallReceipt, Error, Session};
use wasmparser::*;

pub use config::Config;

/// Executes a transaction in the provided session.
///
/// This function processes the transaction, invoking smart contracts or
/// updating state.
///
/// During the execution the following steps are performed:
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
///
///   Deployment execution may fail for deployment-specific reasons, such as:
///    - contract already deployed
///    - corrupted bytecode
///
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
///
/// # Arguments
/// * `session` - A mutable reference to the session executing the transaction.
/// * `tx` - The transaction to execute.
/// * `config` - The configuration for the execution of the transaction.
///
/// # Returns
/// A result indicating success or failure.
pub fn execute(
    session: &mut Session,
    tx: &Transaction,
    config: &Config,
) -> Result<CallReceipt<Result<Vec<u8>, ContractError>>, Error> {
    // Transaction will be discarded if it is a deployment transaction
    // with gas limit smaller than deploy charge.
    tx.deploy_check(
        config.gas_per_deploy_byte,
        config.min_deploy_gas_price,
        config.min_deploy_points,
    )
    .map_err(|e| Error::Panic(e.legacy_to_string()))?;

    if let Some(contract_deploy) = tx.deploy() {
        match (config.disable_wasm32, config.disable_wasm64) {
            (true, true) => Err(Error::Panic(
                "contract deployment is not enabled in the VM".into(),
            )),
            (true, false) if !is_wasm64(&contract_deploy.bytecode.bytes) => {
                Err(Error::Panic("32-bit wasm is not enabled in the VM".into()))
            }
            (false, true) if is_wasm64(&contract_deploy.bytecode.bytes) => {
                Err(Error::Panic("64-bit wasm is not enabled in the VM".into()))
            }
            _ => Ok(()),
        }?
    }

    if config.disable_3rd_party {
        if let Some(call) = tx.call() {
            if call.contract != TRANSFER_CONTRACT
                && call.contract != STAKE_CONTRACT
            {
                return Err(Error::Panic(
                    "3rd party contracts are not enabled in the VM".into(),
                ));
            }
        }
    }

    let blob_min_charge = tx
        .blob_check(config.gas_per_blob)
        .map_err(|e| Error::Panic(e.legacy_to_string()))?;

    if blob_min_charge.is_some() && !config.with_blob {
        return Err(Error::Panic(
            "Blob processing is not enabled in the VM".into(),
        ));
    }

    if config.with_public_sender {
        let _ = session
            .set_meta(Metadata::PUBLIC_SENDER, tx.moonlight_sender().copied());
    }

    let stripped_tx = tx.blob_to_memo().or(tx.strip_off_bytecode());

    // Spend the inputs and execute the call. If this errors the transaction is
    // unspendable.
    let mut receipt = session
        .call::<_, Result<Vec<u8>, ContractError>>(
            TRANSFER_CONTRACT,
            "spend_and_execute",
            stripped_tx.as_ref().unwrap_or(tx),
            tx.gas_limit(),
        )
        .inspect_err(|_| {
            clear_session(session, config);
        })?;

    // Deploy if this is a deployment transaction and spend part is successful.
    contract_deploy(session, tx, config, &mut receipt);

    // If this is a blob transaction, ensure the gas spent is at least the
    // minimum charge.
    if let Some(blob_min_charge) = blob_min_charge {
        if receipt.gas_spent < blob_min_charge {
            receipt.gas_spent = blob_min_charge;
        }
    }

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

    clear_session(session, config);

    Ok(receipt)
}

fn is_wasm64(bytecode: &[u8]) -> bool {
    for payload in Parser::new(0).parse_all(bytecode).flatten() {
        if let Payload::MemorySection(section) = payload {
            return section
                .into_iter()
                .any(|memory| memory.map_or(false, |m| m.memory64));
        }
    }
    false
}

fn clear_session(session: &mut Session, config: &Config) {
    if config.with_public_sender {
        let _ = session.remove_meta(Metadata::PUBLIC_SENDER);
    }
}

// Contract deployment will fail and charge full gas limit in the
// following cases:
// 1) Transaction gas limit is smaller than deploy charge plus gas used for
//    spending funds.
// 2) Transaction's bytecode's bytes are not consistent with bytecode's hash.
// 3) Deployment fails for deploy-specific reasons like e.g.:
//      - contract already deployed
//      - corrupted bytecode
//      - sufficient gas to spend funds yet insufficient for deployment
fn contract_deploy(
    session: &mut Session,
    tx: &Transaction,
    config: &Config,
    receipt: &mut CallReceipt<Result<Vec<u8>, ContractError>>,
) {
    if let Some(deploy) = tx.deploy() {
        let gas_per_deploy_byte = config.gas_per_deploy_byte;
        let min_deploy_points = config.min_deploy_points;

        let gas_left = tx.gas_limit() - receipt.gas_spent;
        if receipt.data.is_ok() {
            let deploy_charge =
                tx.deploy_charge(gas_per_deploy_byte, min_deploy_points);
            let min_gas_limit = receipt.gas_spent + deploy_charge;
            if gas_left < min_gas_limit {
                receipt.data = Err(ContractError::OutOfGas);
            } else if !verify_bytecode_hash(&deploy.bytecode) {
                receipt.data = Err(ContractError::Panic(
                    "failed bytecode hash check".into(),
                ))
            } else {
                let result = session.deploy_raw(
                    Some(gen_contract_id(
                        &deploy.bytecode.bytes,
                        deploy.nonce,
                        &deploy.owner,
                    )),
                    deploy.bytecode.bytes.as_slice(),
                    deploy.init_args.clone(),
                    deploy.owner.clone(),
                    gas_left,
                );
                match result {
                    // Should the gas spent by the INIT method charged too?
                    Ok(_) => receipt.gas_spent += deploy_charge,
                    Err(err) => {
                        let msg = format!("failed deployment: {err:?}");
                        receipt.data = Err(ContractError::Panic(msg))
                    }
                }
            }
        }
    }
}

// Verifies that the stored contract bytecode hash is correct.
fn verify_bytecode_hash(bytecode: &ContractBytecode) -> bool {
    let computed: [u8; 32] = blake3::hash(bytecode.bytes.as_slice()).into();

    bytecode.hash == computed
}

/// Generates a unique identifier for a smart contract.
///
/// # Arguments
/// * 'bytes` - The contract bytecode.
/// * `nonce` - A unique nonce.
/// * `owner` - The contract-owner.
///
/// # Returns
/// A unique [`ContractId`].
///
/// # Panics
/// Panics if [blake2b-hasher] doesn't produce a [`CONTRACT_ID_BYTES`]
/// bytes long hash.
///
/// [blake2b-hasher]: [`blake2b_simd::Params.finalize`]
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
    use alloc::vec;

    // the `unused_crate_dependencies` lint complains for dev-dependencies that
    // are only used in integration tests, so adding this work-around here
    use ff as _;
    use hex as _;
    use once_cell as _;
    use rand::rngs::StdRng;
    use rand::{RngCore, SeedableRng};

    use super::*;

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
