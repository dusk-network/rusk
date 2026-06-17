// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![deny(clippy::all)]

use dusk_core::signatures::bls::{
    PublicKey as AccountPublicKey, SecretKey as AccountSecretKey,
};
use dusk_core::transfer::data::TransactionData;
use dusk_core::transfer::moonlight::Transaction as MoonlightTransaction;
use dusk_core::transfer::{TRANSFER_CONTRACT, Transaction};
use dusk_vm::{ContractData, Error, ExecutionConfig, VM, execute};
use rand::SeedableRng;
use rand::rngs::StdRng;

const CHAIN_ID: u8 = 0xFA;
const TX_GAS_LIMIT: u64 = 1_000_000;
const TX_GAS_PRICE: u64 = 1;

#[test]
fn execute_returns_error_when_refund_call_fails() {
    let _host_query_policy_guard = dusk_vm::host_queries::set_host_query_policy(
        dusk_vm::host_queries::HostQueryPolicy::from_versions(
            dusk_vm::host_queries::plonk_version(),
            dusk_vm::host_queries::HardFork::Aegis,
        ),
    );
    let vm = VM::ephemeral().expect("Instantiating VM should succeed");
    let mut session = vm.genesis_session(CHAIN_ID);

    session
        .deploy::<_, (), _>(
            include_bytes!("../../rusk-recovery/assets/transfer_contract.wasm"),
            ContractData::builder()
                .contract_id(TRANSFER_CONTRACT)
                .owner(vec![]),
            u64::MAX,
        )
        .expect("Deploying transfer contract should succeed");

    let mut rng = StdRng::seed_from_u64(0xfeed);
    let sender_sk = AccountSecretKey::random(&mut rng);
    let sender_pk = AccountPublicKey::from(&sender_sk);
    let receiver_sk = AccountSecretKey::random(&mut rng);
    let receiver_pk = AccountPublicKey::from(&receiver_sk);
    let refund_sk = AccountSecretKey::random(&mut rng);
    let refund_pk = AccountPublicKey::from(&refund_sk);
    let transfer_value = 17;

    session
        .call::<(AccountPublicKey, u64), ()>(
            TRANSFER_CONTRACT,
            "add_account_balance",
            &(sender_pk, transfer_value + TX_GAS_LIMIT * TX_GAS_PRICE),
            u64::MAX,
        )
        .expect("Funding sender should succeed");
    session
        .call::<(AccountPublicKey, u64), ()>(
            TRANSFER_CONTRACT,
            "add_account_balance",
            &(refund_pk, u64::MAX),
            u64::MAX,
        )
        .expect("Funding refund account should succeed");

    let base = session.commit().expect("Committing should succeed");
    let mut session = vm
        .session(base, CHAIN_ID, 0)
        .expect("Instantiating session should succeed");

    // Use a second Moonlight account as the refund target and prefill it to
    // force the real transfer contract's refund credit to overflow.
    let tx: Transaction = MoonlightTransaction::new_with_refund(
        &sender_sk,
        &refund_pk,
        Some(receiver_pk),
        transfer_value,
        0,
        TX_GAS_LIMIT,
        TX_GAS_PRICE,
        1,
        CHAIN_ID,
        Option::<TransactionData>::None,
    )
    .expect("Creating moonlight transaction should succeed")
    .into();

    let err = execute(&mut session, &tx, &ExecutionConfig::default())
        .expect_err("Execution should fail when refunding fails");

    assert!(matches!(
        err,
        dusk_vm::ExecutionError::FailedRefund(Error::Panic(msg))
            if msg.contains("overflow")
    ));
}
