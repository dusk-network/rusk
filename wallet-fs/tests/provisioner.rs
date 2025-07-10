// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fs;
use std::path::PathBuf;

use anyhow::anyhow;
use dusk_bytes::Serializable;
use dusk_core::signatures::bls::{
    PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
};
use rand::SeedableRng;
use rand::rngs::StdRng;
use tempfile::tempdir;
use wallet_fs::provisioner::{load_keys, save_consensus_keys};

#[test]
fn test_save_load_consensus_keys() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;

    let mut rng = StdRng::seed_from_u64(64);
    let sk = BlsSecretKey::random(&mut rng);
    let pk = BlsPublicKey::from(&sk);
    let pwd = "password";

    save_consensus_keys(dir.path(), "consensus", &pk, &sk, pwd)?;
    let keys_path = dir.path().join("consensus.keys");
    let (loaded_sk, loaded_pk) = load_keys(
        keys_path
            .to_str()
            .ok_or(anyhow!("Failed to convert path to string"))?,
        &pwd,
    )?;
    let pk_bytes = fs::read(dir.path().join("consensus.cpk"))?;
    let pk_bytes: [u8; BlsPublicKey::SIZE] = pk_bytes
        .try_into()
        .map_err(|_| anyhow!("Invalid BlsPublicKey bytes"))?;
    let loaded_cpk = BlsPublicKey::from_bytes(&pk_bytes)
        .map_err(|err| anyhow!("{err:?}"))?;

    assert_eq!(loaded_sk, sk);
    assert_eq!(loaded_pk, pk);
    assert_eq!(loaded_cpk, pk);

    Ok(())
}

#[test]
fn test_can_still_load_keys_saved_by_wallet_impl()
-> Result<(), Box<dyn std::error::Error>> {
    // test-data/wallet-generated-consensus-keys contains consensus keys
    // exported by the former rusk-wallet implementation to save consensus
    // keys.
    // This test checks if what is saved by the former implementation
    // is still loaded correctly.
    let mut rng = StdRng::seed_from_u64(64);
    let sk = BlsSecretKey::random(&mut rng);
    let pk = BlsPublicKey::from(&sk);

    let pwd = "password".to_string();
    let wallet_gen_keys_path = get_wallet_gen_consensus_keys_path();
    let temp_dir = tempdir()?;
    let keys_path = temp_dir.path().join("consensus.keys");
    fs::copy(&wallet_gen_keys_path, &keys_path)?;

    let (loaded_sk, loaded_pk) = load_keys(keys_path.to_str().unwrap(), &pwd)?;

    assert_eq!(loaded_sk, sk);
    assert_eq!(loaded_pk, pk);

    let old_keys_path = temp_dir.path().join("consensus.keys.old");
    assert!(old_keys_path.exists(), "Old keys path should exist");

    Ok(())
}

fn get_wallet_gen_consensus_keys_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("test-data")
        .join("wallet-generated-consensus-keys")
        .join("consensus.keys")
}
