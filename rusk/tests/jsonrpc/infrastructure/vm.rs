// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg(feature = "chain")]

use dusk_core::signatures::bls::{PublicKey as BlsPublicKey, SecretKey};
use dusk_core::transfer::moonlight::AccountData;
use rusk::jsonrpc::infrastructure::error::VmError;
use rusk::jsonrpc::infrastructure::vm::{RuskVmAdapter, VmAdapter};
use rusk::node::Rusk as NodeRusk;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::sync::broadcast;

// Import for state deployment
use rusk_recovery_tools::state;

#[cfg(feature = "archive")]
use node::archive::Archive;

use crate::jsonrpc::utils::MockVmAdapter;

// Helper to create a basic NodeRusk instance for testing
async fn create_test_node_rusk() -> Arc<NodeRusk> {
    let tmp = tempdir().expect("Failed to create temp dir");
    let state_dir = tmp.path(); // Keep as Path for deploy

    // --- Initialize State Directory --- START ---
    let snapshot_str = include_str!("../../../tests/config/init.toml"); // Assuming path relative to this file
    let snapshot = toml::from_str(snapshot_str)
        .expect("Cannot deserialize default init.toml snapshot");
    let dusk_key = *rusk::DUSK_CONSENSUS_KEY;
    state::deploy(state_dir, &snapshot, dusk_key, |_| {})
        .expect("Deploying initial state should succeed");
    // --- Initialize State Directory --- END ---

    let vm_config = rusk::node::RuskVmConfig::new();
    let (event_sender, _) = broadcast::channel(1);

    #[cfg(feature = "archive")]
    let archive = Archive::create_or_open(state_dir).await; // Use state_dir for archive path too

    Arc::new(
        NodeRusk::new(
            state_dir,
            250, // <-- Use 0xFA (250) as chain_id instead of 0
            vm_config,
            1000,   // min_gas_limit
            100000, // feeder_gas_limit
            event_sender,
            #[cfg(feature = "archive")]
            archive,
        )
        .expect("Failed to create NodeRusk"),
    )
}

#[tokio::test]
// TODO: Unignore when a state snapshot including TRANSFER_CONTRACT deployment
// is available. Read the comment inside the function for more details.
#[ignore = "init.toml doesn't deploy TRANSFER_CONTRACT"]
async fn test_rusk_vm_adapter_get_chain_id() {
    // Await the async helper function
    let node_rusk = create_test_node_rusk().await;
    let adapter = RuskVmAdapter::new(node_rusk.clone());

    let result = adapter.get_chain_id().await;

    // This assertion fails because the default init.toml doesn't deploy
    // TRANSFER_CONTRACT, which Rusk::chain_id queries internally.
    // Test remains ignored, but we update the assertion to check the expected
    // value (250) derived from the `create_test_node_rusk` setup.
    assert_eq!(result, Ok(250));
}

#[tokio::test]
async fn test_rusk_vm_adapter_get_account_data() {
    // Await the async helper function
    let node_rusk = create_test_node_rusk().await; // <-- Added .await
    let adapter = RuskVmAdapter::new(node_rusk.clone());

    // Create a dummy public key for testing
    // NOTE: This account likely won't exist in the default test NodeRusk state.
    // The underlying `node_rusk.account()` call is expected to fail (e.g., with
    // a state query error).
    let sk = SecretKey::random(&mut rand::thread_rng());
    let pk = BlsPublicKey::from(&sk);

    let result = adapter.get_account_data(&pk).await;

    // Expect an error because the account shouldn't exist in a fresh state
    assert!(result.is_err());
    match result.err().unwrap() {
        VmError::QueryFailed(_) => { /* Expected error type */ }
        e => panic!("Unexpected error type: {:?}", e),
    }

    // TODO: A more thorough test would involve:
    // 1. Setting up the NodeRusk state with a known account and balance/nonce.
    // 2. Querying that known account's public key.
    // 3. Asserting that the returned Ok(AccountData) matches the setup.
    // This requires more intricate test setup for NodeRusk state.
}

#[tokio::test]
async fn test_vm_adapter_default_methods() {
    // 1. Setup MockVmAdapter
    let mut mock_adapter = MockVmAdapter::default();
    let sk = SecretKey::random(&mut rand::thread_rng());
    let pk = BlsPublicKey::from(&sk);

    let expected_balance = 12345_u64;
    let expected_nonce = 42_u64;
    let account_data = AccountData {
        balance: expected_balance,
        nonce: expected_nonce,
    };

    // Configure the mock to return specific AccountData for our pk
    // Use Vec instead of HashMap
    let data_vec = vec![(pk, account_data)]; // <-- Changed to Vec::new()
    mock_adapter.account_data = Some(data_vec);

    // 2. Test get_account_balance (Default Method)
    let balance_result = mock_adapter.get_account_balance(&pk).await;
    assert!(balance_result.is_ok());
    assert_eq!(balance_result.unwrap(), expected_balance);

    // 3. Test get_account_nonce (Default Method)
    let nonce_result = mock_adapter.get_account_nonce(&pk).await;
    assert!(nonce_result.is_ok());
    assert_eq!(nonce_result.unwrap(), expected_nonce);

    // 4. Test with a different PK not in the map (should use mock's default)
    let sk2 = SecretKey::random(&mut rand::thread_rng());
    let pk2 = BlsPublicKey::from(&sk2);

    let balance_result_default = mock_adapter.get_account_balance(&pk2).await;
    assert!(balance_result_default.is_ok());
    assert_eq!(balance_result_default.unwrap(), 0); // Mock default balance is 0

    let nonce_result_default = mock_adapter.get_account_nonce(&pk2).await;
    assert!(nonce_result_default.is_ok());
    assert_eq!(nonce_result_default.unwrap(), 0); // Mock default nonce is 0
}
