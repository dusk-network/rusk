// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bls12_381::BlsScalar;
use dusk_pki::SecretSpendKey;
use dusk_plonk::prelude::*;
use lazy_static::lazy_static;
use profile_tooling::CircuitLoader;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

lazy_static! {
    static ref PUB_PARAMS: PublicParameters = {
        match rusk_profile::get_common_reference_string() {
            Ok(buff) if rusk_profile::verify_common_reference_string(&buff) => unsafe {
                info!("Got the CRS from cache");

                PublicParameters::from_slice_unchecked(&buff[..])
            },

            _ => {
                info!("New CRS needs to be generated and cached");

                let mut rng = StdRng::seed_from_u64(0xbeef);

                let pp = PublicParameters::setup(1 << 17, &mut rng)
                    .expect("Cannot initialize Public Parameters");

                info!("Public Parameters initialized");

                rusk_profile::set_common_reference_string(
                    pp.to_raw_var_bytes(),
                )
                .expect("Unable to write the CRS");

                pp
            }
        }
    };
}

/// Buildfile for the rusk crate.
///
/// Main goals of the file at the moment are:
/// 1. Compile the `.proto` files for tonic.
/// 2. Get the version of the crate and some extra info to
/// support the `-v` argument properly.
/// 3. Compile the contract-related circuits.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Ensure we run the build script again even if we change just the build.rs
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=path/to/Cargo.lock");

    // Get crate version + commit + toolchain for `-v` arg support.
    println!(
        "cargo:rustc-env=GIT_HASH={}",
        rustc_tools_util::get_commit_hash().unwrap_or_default()
    );
    println!(
        "cargo:rustc-env=COMMIT_DATE={}",
        rustc_tools_util::get_commit_date().unwrap_or_default()
    );
    println!(
        "cargo:rustc-env=RUSTC_RELEASE_CHANNEL={}",
        rustc_tools_util::get_channel().unwrap_or_default()
    );

    let subscriber = FmtSubscriber::builder()
        // all spans/events with a level higher than TRACE (e.g, debug, info,
        // warn, etc.) will be written to stdout.
        .with_max_level(Level::TRACE)
        // completes the builder.
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    assert!(option_env!("RUSK_PROFILE_PATH").is_some(),
        "RUSK_PROFILE_PATH env var is not set. Please run `source .env` to set it");

    // This will enforce the usage and therefore the cache / generation
    // of the CRS even if it's not used to compiles circuits inside the
    // build script.
    lazy_static::initialize(&PUB_PARAMS);

    // Compile protos for tonic
    tonic_build::compile_protos("../schema/state.proto")?;

    // Run the rusk-profile Circuit-keys checks
    use transfer::*;

    profile_tooling::run_circuit_keys_checks(vec![
        &StctCircuitLoader {},
        &StcoCircuitLoader {},
        &WftCircuitLoader {},
        &WfoCircuitLoader {},
        &ExecuteOneZeroCircuitLoader {},
        &ExecuteOneOneCircuitLoader {},
        &ExecuteOneTwoCircuitLoader {},
        &ExecuteTwoZeroCircuitLoader {},
        &ExecuteTwoOneCircuitLoader {},
        &ExecuteTwoTwoCircuitLoader {},
        &ExecuteThreeZeroCircuitLoader {},
        &ExecuteThreeOneCircuitLoader {},
        &ExecuteThreeTwoCircuitLoader {},
        &ExecuteFourZeroCircuitLoader {},
        &ExecuteFourOneCircuitLoader {},
        &ExecuteFourTwoCircuitLoader {},
    ])?;

    Ok(())
}

mod transfer {
    use super::*;
    use phoenix_core::{Message, Note};
    use std::convert::{TryFrom, TryInto};
    use transfer_circuits::*;

    pub struct StctCircuitLoader;
    impl CircuitLoader for StctCircuitLoader {
        fn circuit_id(&self) -> &[u8; 32] {
            &SendToContractTransparentCircuit::CIRCUIT_ID
        }

        fn circuit_name(&self) -> &'static str {
            "STCT"
        }

        fn compile_circuit(
            &self,
        ) -> Result<(Vec<u8>, Vec<u8>), Box<dyn std::error::Error>> {
            let pub_params = &PUB_PARAMS;
            let rng = &mut StdRng::seed_from_u64(0xbeef);

            let ssk = SecretSpendKey::random(rng);
            let psk = ssk.public_spend_key();

            let address = BlsScalar::random(rng);

            let value = 100;
            let blinder = JubJubScalar::random(rng);

            let note = Note::obfuscated(rng, &psk, value, blinder);
            let (mut fee, crossover) = note
                .try_into()
                .expect("Failed to convert note into fee/crossover pair!");
            fee.gas_limit = 5;
            fee.gas_price = 1;

            let signature = SendToContractTransparentCircuit::sign(
                rng, &ssk, &fee, &crossover, value, &address,
            );

            let mut circuit = SendToContractTransparentCircuit::new(
                &fee, &crossover, value, blinder, address, signature,
            );

            let (pk, vd) = circuit.compile(pub_params)?;
            Ok((pk.to_var_bytes(), vd.to_var_bytes()))
        }
    }

    pub struct StcoCircuitLoader;
    impl CircuitLoader for StcoCircuitLoader {
        fn circuit_id(&self) -> &[u8; 32] {
            &SendToContractObfuscatedCircuit::CIRCUIT_ID
        }

        fn circuit_name(&self) -> &'static str {
            "STCO"
        }

        fn compile_circuit(
            &self,
        ) -> Result<(Vec<u8>, Vec<u8>), Box<dyn std::error::Error>> {
            let pub_params = &PUB_PARAMS;
            let rng = &mut StdRng::seed_from_u64(0xbeef);

            let c_ssk = SecretSpendKey::random(rng);
            let c_psk = c_ssk.public_spend_key();

            let value = rng.gen();

            let c_blinder = JubJubScalar::random(rng);
            let c_note = Note::obfuscated(rng, &c_psk, value, c_blinder);

            let (mut fee, crossover) = c_note
                .try_into()
                .expect("Failed to convert note into fee/crossover pair!");

            fee.gas_limit = 5;
            fee.gas_price = 1;

            let m_ssk = SecretSpendKey::random(rng);
            let m_psk = m_ssk.public_spend_key();

            let m_r = JubJubScalar::random(rng);
            let message = Message::new(rng, &m_r, &m_psk, value);
            let m_pk_r = *m_psk.gen_stealth_address(&m_r).pk_r().as_ref();

            let (_, m_blinder) = message
                .decrypt(&m_r, &m_psk)
                .expect("Failed to decrypt message");

            let m_derive_key = DeriveKey::new(false, &m_psk);

            let address = BlsScalar::random(rng);
            let signature = SendToContractObfuscatedCircuit::sign(
                rng, &c_ssk, &fee, &crossover, &message, &address,
            );

            let message = StcoMessage {
                blinder: m_blinder,
                derive_key: m_derive_key,
                message,
                pk_r: m_pk_r,
                r: m_r,
            };

            let crossover = StcoCrossover::new(crossover, c_blinder);

            let mut circuit = SendToContractObfuscatedCircuit::new(
                value, message, crossover, &fee, address, signature,
            );

            let (pk, vd) = circuit.compile(pub_params)?;
            Ok((pk.to_var_bytes(), vd.to_var_bytes()))
        }
    }

    pub struct WftCircuitLoader;
    impl CircuitLoader for WftCircuitLoader {
        fn circuit_id(&self) -> &[u8; 32] {
            &WithdrawFromTransparentCircuit::CIRCUIT_ID
        }

        fn circuit_name(&self) -> &'static str {
            "WFT"
        }

        fn compile_circuit(
            &self,
        ) -> Result<(Vec<u8>, Vec<u8>), Box<dyn std::error::Error>> {
            let pub_params = &PUB_PARAMS;
            let rng = &mut StdRng::seed_from_u64(0xbeef);

            let ssk = SecretSpendKey::random(rng);
            let psk = ssk.public_spend_key();

            let value = rng.gen();
            let blinder = JubJubScalar::random(rng);

            let note = Note::obfuscated(rng, &psk, value, blinder);
            let commitment = *note.value_commitment();

            let mut circuit =
                WithdrawFromTransparentCircuit::new(commitment, value, blinder);

            let (pk, vd) = circuit.compile(pub_params)?;
            Ok((pk.to_var_bytes(), vd.to_var_bytes()))
        }
    }

    pub struct WfoCircuitLoader;
    impl CircuitLoader for WfoCircuitLoader {
        fn circuit_id(&self) -> &[u8; 32] {
            &WithdrawFromObfuscatedCircuit::CIRCUIT_ID
        }

        fn circuit_name(&self) -> &'static str {
            "WFO"
        }

        fn compile_circuit(
            &self,
        ) -> Result<(Vec<u8>, Vec<u8>), Box<dyn std::error::Error>> {
            let pub_params = &PUB_PARAMS;
            let rng = &mut StdRng::seed_from_u64(0xbeef);

            let input = {
                let ssk = SecretSpendKey::random(rng);
                let psk = ssk.public_spend_key();

                let value = 100;
                let r = JubJubScalar::random(rng);
                let message = Message::new(rng, &r, &psk, value);
                let commitment = *message.value_commitment();

                let (_, blinder) = message
                    .decrypt(&r, &psk)
                    .expect("Failed to decrypt message");

                WfoCommitment {
                    blinder,
                    commitment,
                    value,
                }
            };
            let change = {
                let ssk = SecretSpendKey::random(rng);
                let psk = ssk.public_spend_key();

                let value = 25;
                let r = JubJubScalar::random(rng);
                let message = Message::new(rng, &r, &psk, value);
                let pk_r = *psk.gen_stealth_address(&r).pk_r().as_ref();

                let (_, blinder) = message
                    .decrypt(&r, &psk)
                    .expect("Failed to decrypt message");

                let derive_key = DeriveKey::new(false, &psk);

                WfoChange {
                    blinder,
                    derive_key,
                    message,
                    pk_r,
                    r,
                    value,
                }
            };

            let output = {
                let ssk = SecretSpendKey::random(rng);
                let psk = ssk.public_spend_key();

                let value = 75;

                let blinder = JubJubScalar::random(rng);
                let output = Note::obfuscated(rng, &psk, value, blinder);
                let commitment = *output.value_commitment();
                WfoCommitment {
                    blinder,
                    commitment,
                    value,
                }
            };

            let mut circuit = WithdrawFromObfuscatedCircuit {
                input,
                change,
                output,
            };

            let (pk, vd) = circuit.compile(pub_params)?;
            Ok((pk.to_var_bytes(), vd.to_var_bytes()))
        }
    }

    macro_rules! execute_circuit_variant {
        ($c:ident,$b:ident,$s:expr,$i:expr,$o:expr) => {
            pub struct $c;
            impl CircuitLoader for $c {
                fn circuit_id(&self) -> &[u8; 32] {
                    &$b::CIRCUIT_ID
                }

                fn circuit_name(&self) -> &'static str {
                    $s
                }

                fn compile_circuit(
                    &self,
                ) -> Result<(Vec<u8>, Vec<u8>), Box<dyn std::error::Error>>
                {
                    let pub_params = &PUB_PARAMS;
                    let rng = &mut StdRng::seed_from_u64(0xbeef);

                    let tx_hash = BlsScalar::random(rng);

                    let circuit = ExecuteCircuit::create_dummy_circuit(
                        rng, $i, $o, true, tx_hash,
                    )?;
                    let mut circuit = $b::try_from(circuit)?;

                    let (pk, vd) = circuit.compile(&pub_params)?;
                    Ok((pk.to_var_bytes(), vd.to_var_bytes()))
                }
            }
        };
    }

    execute_circuit_variant!(
        ExecuteOneZeroCircuitLoader,
        ExecuteCircuitOneZero,
        "ExecuteOneZero",
        1,
        0
    );
    execute_circuit_variant!(
        ExecuteOneOneCircuitLoader,
        ExecuteCircuitOneOne,
        "ExecuteOneOne",
        1,
        1
    );
    execute_circuit_variant!(
        ExecuteOneTwoCircuitLoader,
        ExecuteCircuitOneTwo,
        "ExecuteOneTwo",
        1,
        2
    );
    execute_circuit_variant!(
        ExecuteTwoZeroCircuitLoader,
        ExecuteCircuitTwoZero,
        "ExecuteTwoZero",
        2,
        0
    );
    execute_circuit_variant!(
        ExecuteTwoOneCircuitLoader,
        ExecuteCircuitTwoOne,
        "ExecuteTwoOne",
        2,
        1
    );
    execute_circuit_variant!(
        ExecuteTwoTwoCircuitLoader,
        ExecuteCircuitTwoTwo,
        "ExecuteTwoTwo",
        2,
        2
    );
    execute_circuit_variant!(
        ExecuteThreeZeroCircuitLoader,
        ExecuteCircuitThreeZero,
        "ExecuteThreeZero",
        3,
        0
    );
    execute_circuit_variant!(
        ExecuteThreeOneCircuitLoader,
        ExecuteCircuitThreeOne,
        "ExecuteThreeOne",
        3,
        1
    );
    execute_circuit_variant!(
        ExecuteThreeTwoCircuitLoader,
        ExecuteCircuitThreeTwo,
        "ExecuteThreeTwo",
        3,
        2
    );
    execute_circuit_variant!(
        ExecuteFourZeroCircuitLoader,
        ExecuteCircuitFourZero,
        "ExecuteFourZero",
        4,
        0
    );
    execute_circuit_variant!(
        ExecuteFourOneCircuitLoader,
        ExecuteCircuitFourOne,
        "ExecuteFourOne",
        4,
        1
    );
    execute_circuit_variant!(
        ExecuteFourTwoCircuitLoader,
        ExecuteCircuitFourTwo,
        "ExecuteFourTwo",
        4,
        2
    );
}

mod profile_tooling {
    use super::*;

    pub trait CircuitLoader {
        fn circuit_id(&self) -> &[u8; 32];

        fn circuit_name(&self) -> &'static str;

        fn compile_circuit(
            &self,
        ) -> Result<(Vec<u8>, Vec<u8>), Box<dyn std::error::Error>>;
    }

    fn clear_outdated_keys(
        loader_list: &[&dyn CircuitLoader],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let id_list: Vec<_> = loader_list
            .iter()
            .map(|loader| loader.circuit_id())
            .cloned()
            .collect();

        Ok(rusk_profile::clean_outdated_keys(&id_list)?)
    }

    fn check_keys_cache(
        loader_list: &[&dyn CircuitLoader],
    ) -> Result<Vec<()>, Box<dyn std::error::Error>> {
        loader_list
            .iter()
            .map(|loader| {
                info!("{} Keys cache checking stage", loader.circuit_name());
                match rusk_profile::keys_for(loader.circuit_id()) {
                    Ok(_) => {
                        info!(
                            "{} already loaded correctly!",
                            loader.circuit_name()
                        );
                        info!("[{}]\n", hex::encode(loader.circuit_id()));
                        Ok(())
                    }
                    _ => {
                        warn!("{} not cached!", loader.circuit_name());
                        info!(
                            "Compiling {} and adding to the cache",
                            loader.circuit_name()
                        );
                        let (pk, vd) = loader.compile_circuit()?;
                        rusk_profile::add_keys_for(
                            loader.circuit_id(),
                            pk,
                            vd,
                        )?;
                        info!(
                            "{} Keys cache checking stage finished",
                            loader.circuit_name()
                        );
                        Ok(())
                    }
                }
            })
            .collect::<Result<Vec<()>, Box<dyn std::error::Error>>>()
    }

    pub fn run_circuit_keys_checks(
        loader_list: Vec<&dyn CircuitLoader>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        clear_outdated_keys(&loader_list)?;
        check_keys_cache(&loader_list).map(|_| ())
    }
}
