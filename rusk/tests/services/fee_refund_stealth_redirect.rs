// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! PoC: Phoenix Fee Refund Stealth Address Redirection
//!
//! The Fee's stealth_address is not committed by the ZK proof. A malicious
//! block producer (or MITM) can replace it with their own address, redirecting
//! the entire gas refund without invalidating the proof.
//!
//! The fix binds `Fee.stealth_address` to the ZK-proven change note
//! (`outputs[1]`) and adds a `phoenix_refund_check()` that rejects any
//! transaction where `fee.stealth_address != outputs[1].stealth_address()`.

use dusk_core::JubJubScalar;
use dusk_core::transfer::Transaction;
use dusk_core::transfer::phoenix::{
    Fee, Payload, PublicKey, SecretKey, Sender,
    Transaction as PhoenixTransaction, TxSkeleton,
};
use dusk_rusk_test::common::state::ExecuteResult;
use dusk_rusk_test::{Result, RuskVmConfig, TestContext};
use ff::Field;
use rand::SeedableRng;
use rand::rngs::StdRng;
use tracing::info;

use crate::common::logger;
use crate::common::state::generator_procedure;

const BLOCK_GAS_LIMIT: u64 = 100_000_000_000;

/// Verifies that a Phoenix transaction with a redirected Fee refund stealth
/// address is rejected.
///
/// A malicious block producer replaces `Fee.stealth_address` with an address
/// they control, keeping `gas_limit * gas_price` unchanged so
/// `phoenix_fee_check()` still passes. The `phoenix_refund_check()` must
/// catch the mismatch with `outputs[1]`.
#[tokio::test(flavor = "multi_thread")]
pub async fn fee_refund_stealth_redirect_poc() -> Result<()> {
    logger();

    let state_toml = include_str!("../config/transfer.toml");
    let vm_config = RuskVmConfig::new().with_block_gas_limit(BLOCK_GAS_LIMIT);
    let tc = TestContext::instantiate(state_toml, vm_config).await?;

    let wallet = tc.wallet();
    let rusk = tc.rusk();
    let mut rng = StdRng::seed_from_u64(0xcafe);

    let initial_balance =
        wallet.get_balance(0).expect("Failed to get balance").value;
    info!("=== Phoenix Fee Refund Stealth Redirect PoC ===");
    info!("Initial balance: {} LUX", initial_balance);

    let receiver_pk = wallet
        .phoenix_public_key(0)
        .expect("Failed to get public key");

    // Step 1: Create a legitimate transaction
    let gas_limit: u64 = 100_000_000;
    let gas_price: u64 = 1;

    let tx = wallet
        .phoenix_transfer(
            &mut rng,
            0,
            &receiver_pk,
            0, // transfer_value
            gas_limit,
            gas_price,
        )
        .expect("Failed to create transaction");

    let phoenix_tx = match &tx {
        Transaction::Phoenix(ptx) => ptx,
        _ => panic!("Expected a Phoenix transaction"),
    };

    info!(
        "Honest fee stealth_address: {:?}",
        phoenix_tx.fee().stealth_address
    );

    // Step 2: Generate an attacker's stealth address
    let attacker_sk = SecretKey::random(&mut rng);
    let attacker_pk = PublicKey::from(&attacker_sk);
    let r = JubJubScalar::random(&mut rng);
    let attacker_sa = attacker_pk.gen_stealth_address(&r);

    info!("Attacker stealth_address: {:?}", attacker_sa);

    // Step 3: Replace Fee.stealth_address with the attacker's address
    //
    // Keep gas_limit and gas_price the same so phoenix_fee_check() passes.
    // Only the stealth_address is tampered — this is the P1.5-2 attack.
    let original_fee = phoenix_tx.fee();
    let tampered_fee = Fee {
        gas_limit: original_fee.gas_limit,
        gas_price: original_fee.gas_price,
        stealth_address: attacker_sa,
        sender: Sender::encrypt(
            attacker_sa.note_pk(),
            &attacker_pk,
            &[
                JubJubScalar::random(&mut rng),
                JubJubScalar::random(&mut rng),
            ],
        ),
    };

    let tx_skeleton = TxSkeleton {
        root: *phoenix_tx.root(),
        nullifiers: phoenix_tx.nullifiers().to_vec(),
        outputs: phoenix_tx.outputs().clone(),
        max_fee: phoenix_tx.max_fee(),
        deposit: phoenix_tx.deposit(),
    };

    let payload = Payload {
        chain_id: phoenix_tx.chain_id(),
        tx_skeleton,
        fee: tampered_fee,
        data: None,
    };

    let malicious_phoenix_tx = PhoenixTransaction::from_payload_and_proof(
        payload,
        phoenix_tx.proof().to_vec(),
    );
    let malicious_tx: Transaction = malicious_phoenix_tx.into();

    // Step 4: Execute — should be rejected by phoenix_refund_check()
    generator_procedure(
        rusk,
        &[malicious_tx],
        2, // block_height
        BLOCK_GAS_LIMIT,
        vec![],
        Some(ExecuteResult {
            executed: 0,
            discarded: 1,
        }),
    )
    .expect(
        "Block generation should succeed (with the malicious tx discarded)",
    );

    // Step 5: Verify no balance change
    let final_balance =
        wallet.get_balance(0).expect("Failed to get balance").value;

    info!("Final balance: {} LUX", final_balance);

    assert!(
        final_balance <= initial_balance,
        "REFUND THEFT: balance changed from {} to {} LUX — \
         the stealth address redirection should have been rejected.",
        initial_balance,
        final_balance,
    );

    Ok(())
}
