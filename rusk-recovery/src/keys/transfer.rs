// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::keys::CircuitLoader;
use crate::keys::PUB_PARAMS;
use dusk_bls12_381::BlsScalar;
use dusk_pki::SecretSpendKey;
use dusk_plonk::prelude::*;
use phoenix_core::{Message, Note};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
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
            ) -> Result<(Vec<u8>, Vec<u8>), Box<dyn std::error::Error>> {
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
