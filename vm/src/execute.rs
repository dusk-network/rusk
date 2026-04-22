// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod config;
pub mod feature;

pub use config::Config;
use dusk_core::abi::{ContractError, Metadata};
use dusk_core::stake::STAKE_CONTRACT;
use dusk_core::transfer::data::{ContractBytecode, gen_contract_id};
use dusk_core::transfer::withdraw::{Withdraw, WithdrawReplayToken};
use dusk_core::transfer::{TRANSFER_CONTRACT, Transaction};
use piecrust::{CallReceipt, Session};
use rkyv::Deserialize;
use wasmparser::*;

use crate::ExecutionError;

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
///    optional contract call in steps 2 or 3. If this fails, a specific error
///    `FailedRefund` is returned, then the tx should be considered
///    unspendable/invalid, and the caller SHALL DO a re-execution of previous
///    transactions.
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
) -> Result<CallReceipt<Result<Vec<u8>, ContractError>>, ExecutionError> {
    tx.phoenix_fee_check()?;

    if config.phoenix_refund_check {
        tx.phoenix_refund_check()?;
    }

    // Transaction will be discarded if it is a deployment transaction
    // with gas limit smaller than deploy charge.
    tx.deploy_check(
        config.gas_per_deploy_byte,
        config.min_deploy_gas_price,
        config.min_deploy_points,
    )?;

    if let Some(contract_deploy) = tx.deploy() {
        let is_wasm64 = is_wasm64(&contract_deploy.bytecode.bytes);
        match (config.disable_wasm32, config.disable_wasm64) {
            (true, true) => Err(ExecutionError::precondition(
                "contract deployment is not enabled in the VM",
            )),
            (true, false) if !is_wasm64 => Err(ExecutionError::precondition(
                "32-bit wasm is not enabled in the VM",
            )),
            (false, true) if is_wasm64 => Err(ExecutionError::precondition(
                "64-bit wasm is not enabled in the VM",
            )),
            _ => Ok(()),
        }?
    }

    if config.disable_3rd_party
        && let Some(call) = tx.call()
        && call.contract != TRANSFER_CONTRACT
        && call.contract != STAKE_CONTRACT
    {
        return Err(ExecutionError::precondition(
            "3rd party contracts are not enabled in the VM",
        ));
    }

    let blob_min_charge = tx.blob_check(config.gas_per_blob)?;

    if blob_min_charge.is_some() && !config.with_blob {
        return Err(ExecutionError::precondition(
            "Blob processing is not enabled in the VM",
        ));
    }

    if config.with_public_sender {
        let _ = session
            .set_meta(Metadata::PUBLIC_SENDER, tx.moonlight_sender().copied());
    }

    let stripped_tx = tx.blob_to_memo().or(tx.strip_off_bytecode());

    // Register a call hook to enforce that Phoenix withdrawal replay tokens
    // carry exactly the same number of nullifiers as the encapsulating
    // transaction. This is a defense-in-depth measure against audit finding
    // P1.6-3 (subset-vs-equality in mint_withdrawal).
    //
    // Gated behind the Boreas hard fork activation height.
    if config.withdrawal_nullifier_check && tx.call().is_some() {
        let tx_nullifier_count = tx.nullifiers().len();
        session.set_call_hook(Box::new(move |callee, fn_name, fn_args| {
            check_withdrawal_nullifiers(
                callee,
                fn_name,
                fn_args,
                tx_nullifier_count,
            )
        }));
    }

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
        })
        .map_err(ExecutionError::from_spend_and_execute)?;

    // Deploy if this is a deployment transaction and spend part is successful.
    contract_deploy(session, tx, config, &mut receipt);

    // If this is a blob transaction, ensure the gas spent is at least the
    // minimum charge.
    if let Some(blob_min_charge) = blob_min_charge
        && receipt.gas_spent < blob_min_charge
    {
        receipt.gas_spent = blob_min_charge;
    }

    // Ensure all gas is consumed if there's an error in the contract call
    if receipt.data.is_err() {
        receipt.gas_spent = receipt.gas_limit;
    }

    // Refund the appropriate amount to the transaction. If this errors, the
    // transaction must be discarded by the caller who is also responsible to
    // revert the state applied during the spend_and_execute.
    let refund_receipt = session
        .call::<_, ()>(
            TRANSFER_CONTRACT,
            "refund",
            &receipt.gas_spent,
            u64::MAX,
        )
        .inspect_err(|_| {
            clear_session(session, config);
        })
        .map_err(ExecutionError::FailedRefund)?;

    receipt.events.extend(refund_receipt.events);

    clear_session(session, config);

    Ok(receipt)
}

fn is_wasm64(bytecode: &[u8]) -> bool {
    for payload in Parser::new(0).parse_all(bytecode).flatten() {
        if let Payload::MemorySection(section) = payload {
            return section
                .into_iter()
                .any(|memory| memory.is_ok_and(|m| m.memory64));
        }
    }
    false
}

fn clear_session(session: &mut Session, config: &Config) {
    if config.with_public_sender {
        let _ = session.remove_meta(Metadata::PUBLIC_SENDER);
    }
    session.clear_call_hook();
}

/// Checks that a withdrawal's Phoenix replay token nullifier count matches the
/// encapsulating transaction's nullifier count.
///
/// Returns `Err` when the nullifier count mismatches or the arguments
/// cannot be deserialized (fail-closed). Returns `Ok(())` for non-withdrawal
/// calls, Moonlight tokens, or matching counts.
fn check_withdrawal_nullifiers(
    callee: &dusk_core::abi::ContractId,
    fn_name: &str,
    fn_args: &[u8],
    tx_nullifier_count: usize,
) -> Result<(), String> {
    if *callee != TRANSFER_CONTRACT || fn_name != "withdraw" {
        return Ok(());
    }
    let Ok(root) = rkyv::check_archived_root::<Withdraw>(fn_args) else {
        return Err("failed to deserialize withdrawal arguments".into());
    };
    let withdraw: Withdraw = match root.deserialize(&mut rkyv::Infallible) {
        Ok(w) => w,
        Err(infallible) => match infallible {},
    };
    if let WithdrawReplayToken::Phoenix(nullifiers) = withdraw.token()
        && nullifiers.len() != tx_nullifier_count
    {
        return Err(format!(
            "nullifier count mismatch: withdrawal has {}, transaction has {}",
            nullifiers.len(),
            tx_nullifier_count,
        ));
    }
    Ok(())
}

// Contract deployment will fail and charge full gas limit in the
// following cases:
// 1) Pre-Boreas: transaction gas limit is smaller than deploy charge plus gas
//    used for spending funds.
// 2) Boreas+: remaining gas after spending funds is smaller than deploy charge.
// 3) Transaction's bytecode's bytes are not consistent with bytecode's hash.
// 4) Deployment fails for deploy-specific reasons like e.g.:
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

        if receipt.data.is_ok() {
            let Ok(deploy_charge) =
                tx.deploy_charge(gas_per_deploy_byte, min_deploy_points)
            else {
                receipt.data =
                    Err(ContractError::Panic("deploy charge overflow".into()));
                return;
            };
            if !is_deploy_gas_sufficient(
                tx.gas_limit(),
                receipt.gas_spent,
                deploy_charge,
                config.deploy_remaining_gas_check,
            ) {
                receipt.data = Err(ContractError::OutOfGas);
            } else if !verify_bytecode_hash(&deploy.bytecode) {
                receipt.data = Err(ContractError::Panic(
                    "failed bytecode hash check".into(),
                ))
            } else {
                let gas_left = tx.gas_limit().saturating_sub(receipt.gas_spent);
                let init_budget = if config.charge_init_gas {
                    gas_left.saturating_sub(deploy_charge)
                } else {
                    gas_left
                };
                let result = session.deploy_raw(
                    Some(gen_contract_id(
                        &deploy.bytecode.bytes,
                        deploy.nonce,
                        &deploy.owner,
                    )),
                    deploy.bytecode.bytes.as_slice(),
                    deploy.init_args.clone(),
                    deploy.owner.clone(),
                    init_budget,
                );
                match result {
                    Ok((_, init_receipt)) => {
                        receipt.gas_spent =
                            receipt.gas_spent.saturating_add(deploy_charge);
                        apply_deploy_init_receipt(
                            receipt,
                            init_receipt,
                            config.charge_init_gas,
                        );
                    }
                    Err(err) => {
                        let msg = format!("failed deployment: {err:?}");
                        receipt.data = Err(ContractError::Panic(msg))
                    }
                }
            }
        }
    }
}

fn apply_deploy_init_receipt(
    receipt: &mut CallReceipt<Result<Vec<u8>, ContractError>>,
    init_receipt: Option<CallReceipt<Vec<u8>>>,
    charge_init_gas: bool,
) {
    if let Some(init_receipt) = init_receipt {
        if charge_init_gas {
            receipt.gas_spent =
                receipt.gas_spent.saturating_add(init_receipt.gas_spent);
        }
        receipt.events.extend(init_receipt.events);
    }
}

fn is_deploy_gas_sufficient(
    gas_limit: u64,
    gas_spent: u64,
    deploy_charge: u64,
    deploy_remaining_gas_check: bool,
) -> bool {
    let gas_left = gas_limit.saturating_sub(gas_spent);

    if deploy_remaining_gas_check {
        gas_left >= deploy_charge
    } else {
        gas_spent
            .checked_add(deploy_charge)
            .is_some_and(|required| gas_left >= required)
    }
}

// Verifies that the stored contract bytecode hash is correct.
fn verify_bytecode_hash(bytecode: &ContractBytecode) -> bool {
    let computed: [u8; 32] = blake3::hash(bytecode.bytes.as_slice()).into();

    bytecode.hash == computed
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use dusk_core::BlsScalar;
    use dusk_core::abi::{ContractId, Event};
    use rand::rngs::StdRng;
    use rand::{RngCore, SeedableRng};
    // Dev-dependencies only used in integration tests trigger the
    // unused_crate_dependencies lint, so we re-import them here.
    use {ff as _, hex as _, once_cell as _};

    use super::*;
    use crate::CallTree;

    #[test]
    fn check_withdrawal_nullifiers_matching_count_passes() {
        let rng = &mut StdRng::seed_from_u64(0xbeef);

        let note_sk = dusk_core::signatures::schnorr::SecretKey::random(rng);
        let note_pk = dusk_core::signatures::schnorr::PublicKey::from(&note_sk);
        let address =
            dusk_core::transfer::phoenix::StealthAddress::from_raw_unchecked(
                *note_pk.as_ref(),
                note_pk,
            );

        let nullifiers = vec![BlsScalar::from(1), BlsScalar::from(2)];
        let withdraw = dusk_core::transfer::withdraw::Withdraw::new(
            rng,
            &note_sk,
            TRANSFER_CONTRACT,
            100,
            dusk_core::transfer::withdraw::WithdrawReceiver::Phoenix(address),
            dusk_core::transfer::withdraw::WithdrawReplayToken::Phoenix(
                nullifiers.clone(),
            ),
        );

        let args =
            rkyv::to_bytes::<_, 4096>(&withdraw).expect("should serialize");

        assert!(
            check_withdrawal_nullifiers(
                &TRANSFER_CONTRACT,
                "withdraw",
                &args,
                nullifiers.len(),
            )
            .is_ok()
        );
    }

    #[test]
    fn check_withdrawal_nullifiers_mismatched_count_rejects() {
        let rng = &mut StdRng::seed_from_u64(0xbeef);

        let note_sk = dusk_core::signatures::schnorr::SecretKey::random(rng);
        let note_pk = dusk_core::signatures::schnorr::PublicKey::from(&note_sk);
        let address =
            dusk_core::transfer::phoenix::StealthAddress::from_raw_unchecked(
                *note_pk.as_ref(),
                note_pk,
            );

        let nullifiers = vec![BlsScalar::from(1), BlsScalar::from(2)];
        let withdraw = dusk_core::transfer::withdraw::Withdraw::new(
            rng,
            &note_sk,
            TRANSFER_CONTRACT,
            100,
            dusk_core::transfer::withdraw::WithdrawReceiver::Phoenix(address),
            dusk_core::transfer::withdraw::WithdrawReplayToken::Phoenix(
                nullifiers,
            ),
        );

        let args =
            rkyv::to_bytes::<_, 4096>(&withdraw).expect("should serialize");

        // Mismatched count: 2 nullifiers in token, but tx has 3
        let err = check_withdrawal_nullifiers(
            &TRANSFER_CONTRACT,
            "withdraw",
            &args,
            3,
        )
        .unwrap_err();
        assert!(
            err.contains("nullifier count mismatch"),
            "expected mismatch message, got: {err}"
        );
        assert!(err.contains("2") && err.contains("3"));
    }

    #[test]
    fn check_withdrawal_nullifiers_ignores_non_withdraw_calls() {
        assert!(
            check_withdrawal_nullifiers(&TRANSFER_CONTRACT, "refund", &[], 5,)
                .is_ok()
        );

        assert!(
            check_withdrawal_nullifiers(
                &ContractId::from_bytes([0xAA; 32]),
                "withdraw",
                &[],
                5,
            )
            .is_ok()
        );
    }

    #[test]
    fn check_withdrawal_nullifiers_rejects_garbage_args() {
        // Garbage bytes that cannot be deserialized as a Withdraw must
        // be rejected (fail-closed).
        let err = check_withdrawal_nullifiers(
            &TRANSFER_CONTRACT,
            "withdraw",
            &[0xDE, 0xAD, 0xBE, 0xEF],
            2,
        )
        .unwrap_err();
        assert!(err.contains("deserialize"));

        // Empty args must also be rejected.
        check_withdrawal_nullifiers(&TRANSFER_CONTRACT, "withdraw", &[], 1)
            .unwrap_err();
    }

    #[test]
    fn check_withdrawal_nullifiers_ignores_moonlight_token() {
        let rng = &mut StdRng::seed_from_u64(0xdead);

        let moonlight_sk = dusk_core::signatures::bls::SecretKey::random(rng);
        let moonlight_pk =
            dusk_core::signatures::bls::PublicKey::from(&moonlight_sk);

        let withdraw = dusk_core::transfer::withdraw::Withdraw::new(
            rng,
            &moonlight_sk,
            TRANSFER_CONTRACT,
            100,
            dusk_core::transfer::withdraw::WithdrawReceiver::Moonlight(
                moonlight_pk,
            ),
            dusk_core::transfer::withdraw::WithdrawReplayToken::Moonlight(42),
        );

        let args =
            rkyv::to_bytes::<_, 4096>(&withdraw).expect("should serialize");

        // Moonlight token — should return Ok even with mismatched count
        assert!(
            check_withdrawal_nullifiers(
                &TRANSFER_CONTRACT,
                "withdraw",
                &args,
                999,
            )
            .is_ok()
        );
    }

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

    #[test]
    fn deploy_gas_check_matches_prefork_and_boreas_rules() {
        for (gas_limit, gas_spent, deploy_charge, boreas, expected) in [
            (10_000_000, 3_000_000, 5_000_000, false, false),
            (10_000_000, 3_000_000, 5_000_000, true, true),
            (7_000_000, 3_000_000, 5_000_000, false, false),
            (7_000_000, 3_000_000, 5_000_000, true, false),
            (u64::MAX, u64::MAX, 1, false, false),
        ] {
            assert_eq!(
                is_deploy_gas_sufficient(
                    gas_limit,
                    gas_spent,
                    deploy_charge,
                    boreas,
                ),
                expected,
            );
        }
    }

    #[test]
    fn deploy_init_events_are_preserved_before_and_after_boreas() {
        let init_event = Event {
            source: ContractId::from_bytes([7; 32]),
            topic: "runtime_update".into(),
            data: vec![1, 2, 3, 4],
        };
        let build_init_receipt = || CallReceipt {
            gas_spent: 123,
            gas_limit: 999,
            events: vec![init_event.clone()],
            call_tree: CallTree::default(),
            data: Vec::new(),
        };

        let mut prefork_receipt = CallReceipt {
            gas_spent: 10,
            gas_limit: 1000,
            events: vec![],
            call_tree: CallTree::default(),
            data: Ok(Vec::new()),
        };
        apply_deploy_init_receipt(
            &mut prefork_receipt,
            Some(build_init_receipt()),
            false,
        );
        assert_eq!(prefork_receipt.gas_spent, 10);
        assert_eq!(prefork_receipt.events, vec![init_event.clone()]);

        let mut boreas_receipt = CallReceipt {
            gas_spent: 10,
            gas_limit: 1000,
            events: vec![],
            call_tree: CallTree::default(),
            data: Ok(Vec::new()),
        };
        apply_deploy_init_receipt(
            &mut boreas_receipt,
            Some(build_init_receipt()),
            true,
        );
        assert_eq!(boreas_receipt.gas_spent, 133);
        assert_eq!(boreas_receipt.events, vec![init_event]);
    }
}
