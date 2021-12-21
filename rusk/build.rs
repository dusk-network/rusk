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

                use rand::rngs::StdRng;
                use rand::SeedableRng;

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
            let rng = &mut rand::thread_rng();

            let c_ssk = SecretSpendKey::random(rng);
            let c_vk = c_ssk.view_key();
            let c_psk = c_ssk.public_spend_key();

            let c_address = BlsScalar::random(rng);

            let c_value = 100;
            let c_blinding_factor = JubJubScalar::random(rng);

            let c_note =
                Note::obfuscated(rng, &c_psk, c_value, c_blinding_factor);
            let (mut fee, crossover) = c_note
                .try_into()
                .expect("Failed to convert note into fee/crossover pair!");

            fee.gas_limit = 5;
            fee.gas_price = 1;

            let c_signature = SendToContractTransparentCircuit::sign(
                rng, &c_ssk, &fee, &crossover, c_value, &c_address,
            );

            let mut circuit = SendToContractTransparentCircuit::new(
                fee,
                crossover,
                &c_vk,
                c_address,
                c_signature,
            )
            .expect("Failed to create STCT circuit!");

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
            let rng = &mut rand::thread_rng();

            let ssk = SecretSpendKey::random(rng);
            let vk = ssk.view_key();
            let psk = ssk.public_spend_key();

            let c_address = BlsScalar::random(rng);

            let c_value = 100;
            let c_blinding_factor = JubJubScalar::random(rng);
            let c_note =
                Note::obfuscated(rng, &psk, c_value, c_blinding_factor);
            let (mut fee, crossover) = c_note
                .try_into()
                .expect("Failed to convert note into fee/crossover pair!");

            fee.gas_limit = 5;
            fee.gas_price = 1;

            let message_r = JubJubScalar::random(rng);
            let message_value = 100;
            let message = Message::new(rng, &message_r, &psk, message_value);

            let c_signature = SendToContractObfuscatedCircuit::sign(
                rng, &ssk, &fee, &crossover, &message, &c_address,
            );

            let mut circuit = SendToContractObfuscatedCircuit::new(
                fee,
                crossover,
                &vk,
                c_signature,
                true,
                message,
                &psk,
                message_r,
                c_address,
            )
            .expect("Failed to generate circuit!");

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
            let rng = &mut rand::thread_rng();

            let ssk = SecretSpendKey::random(rng);
            let vk = ssk.view_key();
            let psk = ssk.public_spend_key();

            let value = 100;
            let blinding_factor = JubJubScalar::random(rng);

            let note = Note::obfuscated(rng, &psk, value, blinding_factor);

            let mut circuit =
                WithdrawFromTransparentCircuit::new(&note, Some(&vk))
                    .expect("Failed to create WFT circuit!");

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
            let rng = &mut rand::thread_rng();

            let m_r = JubJubScalar::random(rng);
            let m_ssk = SecretSpendKey::random(rng);
            let m_psk = m_ssk.public_spend_key();
            let m_value = 100;
            let m = Message::new(rng, &m_r, &m_psk, m_value);

            let c_r = JubJubScalar::random(rng);
            let c_ssk = SecretSpendKey::random(rng);
            let c_psk = c_ssk.public_spend_key();
            let c_value = 25;
            let c = Message::new(rng, &c_r, &c_psk, c_value);

            let o_ssk = SecretSpendKey::random(rng);
            let o_vk = o_ssk.view_key();
            let o_psk = o_ssk.public_spend_key();
            let o_value = 75;
            let o_blinding_factor = JubJubScalar::random(rng);
            let o = Note::obfuscated(rng, &o_psk, o_value, o_blinding_factor);

            let input = CircuitValueOpening::from_message(&m, &m_psk, &m_r)
                .expect("Failed to generate WFO input");

            let change = WithdrawFromObfuscatedChange::new(c, c_r, c_psk, true)
                .expect("Failed to generate WFO change");

            let output = CircuitValueOpening::from_note(&o, Some(&o_vk))
                .expect("Failed to generate WFO output");

            let mut circuit =
                WithdrawFromObfuscatedCircuit::new(input, change, output);

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
                    let rng = &mut rand::thread_rng();

                    let circuit = ExecuteCircuit::create_dummy_circuit(
                        rng, $i, $o, true,
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
