// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use aes::Aes256;
use blake3::Hash;
use block_modes::block_padding::Pkcs7;
use block_modes::{BlockMode, Cbc};
use consensus::commons::{RoundUpdate, Seed};
use consensus::consensus::Consensus;
use consensus::contract_state::{
    CallParams, Error, Operations, Output, StateRoot,
};
use consensus::user::provisioners::{Provisioners, DUSK};
use consensus::util::pending_queue::PendingQueue;
use consensus::util::pubkey::ConsensusPublicKey;
use dusk_bls12_381_sign::SecretKey;
use dusk_bytes::DeserializableSlice;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{oneshot, Mutex};

pub fn run_main_loop(
    provisioners_num: usize,
    prov_id: usize,
    inbound: PendingQueue,
    outbound: PendingQueue,
    agr_inbound: PendingQueue,
    agr_outbound: PendingQueue,
) {
    // Initialize N hard-coded provisioners
    let keys = load_provisioners_keys(provisioners_num);
    let provisioners = generate_provisioners_from_keys(keys.clone());

    spawn_consensus_in_thread_pool(
        keys[prov_id].clone(),
        provisioners,
        inbound,
        outbound,
        agr_inbound,
        agr_outbound,
    );
}

/// spawn_node runs a separate thread-pool (tokio::runtime) that drives a single instance of consensus.
fn spawn_consensus_in_thread_pool(
    keys: (SecretKey, ConsensusPublicKey),
    p: Provisioners,
    inbound_msgs: PendingQueue,
    outbound_msgs: PendingQueue,
    agr_inbound_queue: PendingQueue,
    agr_outbound_queue: PendingQueue,
) {
    let _ = std::thread::spawn(move || {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2 + consensus::config::ACCUMULATOR_WORKERS_AMOUNT)
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let mut c = Consensus::new(
                    inbound_msgs,
                    outbound_msgs,
                    agr_inbound_queue,
                    agr_outbound_queue,
                    Arc::new(Mutex::new(Executor {})),
                );

                let mut cumulative_block_time = 0f64;
                let prev_seed = Seed::new([0u8; 48]);
                // Run consensus for N rounds
                for i in 1..1000 {
                    let (_cancel_tx, cancel_rx) = oneshot::channel::<i32>();

                    let before = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs();

                    let _ = c
                        .spin(
                            RoundUpdate::new(
                                i,
                                keys.1.clone(),
                                keys.0,
                                prev_seed,
                            ),
                            p.clone(),
                            cancel_rx,
                        )
                        .await;

                    // Calc block time
                    let block_time = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                        - before;
                    cumulative_block_time += block_time as f64;
                    let average_block_time =
                        cumulative_block_time / ((i + 1) as f64);
                    let average_block_time =
                        (average_block_time * 100f64).round() / 100f64;
                    tracing::info!(
                        bls_key = keys.1.encode_short_hex(),
                        round = i,
                        block_time,
                        average_block_time,
                    );
                }
            });
    });
}

pub struct Executor {}
impl Operations for Executor {
    fn verify_state_transition(
        &self,
        _params: CallParams,
    ) -> Result<StateRoot, Error> {
        Ok([0; 32])
    }

    fn execute_state_transition(
        &self,
        _params: CallParams,
    ) -> Result<Output, Error> {
        Ok(Output::default())
    }

    fn accept(&self, _params: CallParams) -> Result<Output, Error> {
        Ok(Output::default())
    }

    fn finalize(&self, _params: CallParams) -> Result<Output, Error> {
        Ok(Output::default())
    }

    fn get_state_root(&self) -> Result<StateRoot, Error> {
        Ok([0; 32])
    }
}

/// Fetches BLS public and secret keys from an encrypted consensus keys file.
///
/// Panics on any error.
pub fn fetch_blskeys_from_file(
    path: PathBuf,
    pwd: Hash,
) -> Option<(
    dusk_bls12_381_sign::PublicKey,
    dusk_bls12_381_sign::SecretKey,
)> {
    use serde::Deserialize;
    type Aes256Cbc = Cbc<Aes256, Pkcs7>;

    /// Bls key pair helper structure
    #[derive(Deserialize)]
    struct BlsKeyPair {
        secret_key_bls: String,
        public_key_bls: String,
    }

    // attempt to load and decode wallet
    let ciphertext =
        fs::read(&path).expect("path should be valid consensus keys file");

    // Decrypt
    let iv = &ciphertext[..16];
    let enc = &ciphertext[16..];

    let cipher =
        Aes256Cbc::new_from_slices(pwd.as_bytes(), iv).expect("valid data");
    let bytes = cipher.decrypt_vec(enc).expect("pwd should be valid");

    let keys: BlsKeyPair =
        serde_json::from_slice(&bytes).expect("keys files should contain json");

    let sk = dusk_bls12_381_sign::SecretKey::from_slice(
        &base64::decode(keys.secret_key_bls).expect("sk should be base64")[..],
    )
    .expect("sk should be valid");

    let pk = dusk_bls12_381_sign::PublicKey::from_slice(
        &base64::decode(keys.public_key_bls).expect("pk should be base64")[..],
    )
    .expect("pk should be valid");

    Some((pk, sk))
}

/// Loads wallet files from $DUSK_WALLET_DIR and returns a vector of all loaded consensus keys.
///
/// It reads RUSK_WALLET_PWD var to unlock wallet files.
fn load_provisioners_keys(n: usize) -> Vec<(SecretKey, ConsensusPublicKey)> {
    let mut keys = vec![];

    let dir = std::env::var("DUSK_WALLET_DIR").unwrap();
    let pwd = std::env::var("DUSK_CONSENSUS_KEYS_PASS").unwrap();

    let pwd = blake3::hash(pwd.as_bytes());

    for i in 0..n {
        let mut path = dir.clone();
        path.push_str(&format!("node_{}.keys", i));
        let path_buf = PathBuf::from(path);

        let (pk, sk) = fetch_blskeys_from_file(path_buf, pwd)
            .expect("should be valid file");

        keys.push((sk, ConsensusPublicKey::new(pk)));
    }

    keys
}

fn generate_provisioners_from_keys(
    keys: Vec<(SecretKey, ConsensusPublicKey)>,
) -> Provisioners {
    let minimum_stake = 1000 * DUSK * 10; // TODO: file issues

    let mut p = Provisioners::new();

    for (_, (_, pk)) in keys.into_iter().enumerate() {
        p.add_member_with_value(pk, minimum_stake);
    }

    p
}
