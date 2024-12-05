// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unused_crate_dependencies)]
#![deny(unused_extern_crates)]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

use alloc::format;
use alloc::vec::Vec;

use dusk_bytes::Serializable;
use dusk_plonk::prelude::Prover as PlonkProver;
use execution_core::transfer::phoenix::{
    Prove, TxCircuit, TxCircuitVec, NOTES_TREE_DEPTH,
};
use execution_core::Error;
use once_cell::sync::Lazy;

static TX_CIRCUIT_1_2_PROVER: Lazy<PlonkProver> =
    Lazy::new(|| fetch_prover("TxCircuitOneTwo"));

static TX_CIRCUIT_2_2_PROVER: Lazy<PlonkProver> =
    Lazy::new(|| fetch_prover("TxCircuitTwoTwo"));

static TX_CIRCUIT_3_2_PROVER: Lazy<PlonkProver> =
    Lazy::new(|| fetch_prover("TxCircuitThreeTwo"));

static TX_CIRCUIT_4_2_PROVER: Lazy<PlonkProver> =
    Lazy::new(|| fetch_prover("TxCircuitFourTwo"));

#[derive(Debug, Default)]
pub struct LocalProver;

impl Prove for LocalProver {
    fn prove(&self, tx_circuit_vec_bytes: &[u8]) -> Result<Vec<u8>, Error> {
        let tx_circuit_vec = TxCircuitVec::from_slice(tx_circuit_vec_bytes)?;

        #[cfg(not(feature = "no_random"))]
        let rng = &mut rand::rngs::OsRng;

        #[cfg(feature = "no_random")]
        use rand::{rngs::StdRng, SeedableRng};
        #[cfg(feature = "no_random")]
        let rng = &mut StdRng::seed_from_u64(0xbeef);

        #[cfg(feature = "debug")]
        tracing::info!(
            "tx_circuit_vec:\n{}",
            hex::encode(tx_circuit_vec_bytes)
        );

        let (proof, _pi) = match tx_circuit_vec.input_notes_info.len() {
            1 => TX_CIRCUIT_1_2_PROVER
                .prove(rng, &create_circuit::<1>(tx_circuit_vec)?)
                .map_err(|e| Error::PhoenixProver(format!("{e:?}")))?,
            2 => TX_CIRCUIT_2_2_PROVER
                .prove(rng, &create_circuit::<2>(tx_circuit_vec)?)
                .map_err(|e| Error::PhoenixProver(format!("{e:?}")))?,
            3 => TX_CIRCUIT_3_2_PROVER
                .prove(rng, &create_circuit::<3>(tx_circuit_vec)?)
                .map_err(|e| Error::PhoenixProver(format!("{e:?}")))?,
            4 => TX_CIRCUIT_4_2_PROVER
                .prove(rng, &create_circuit::<4>(tx_circuit_vec)?)
                .map_err(|e| Error::PhoenixProver(format!("{e:?}")))?,
            _ => return Err(Error::InvalidData),
        };

        Ok(proof.to_bytes().to_vec())
    }
}

fn fetch_prover(circuit_name: &str) -> PlonkProver {
    let circuit_profile = rusk_profile::Circuit::from_name(circuit_name)
        .unwrap_or_else(|_| {
            panic!(
                "There should be tx-circuit data stored for {}",
                circuit_name
            )
        });
    let pk = circuit_profile.get_prover().unwrap_or_else(|_| {
        panic!("there should be a prover key stored for {}", circuit_name)
    });

    PlonkProver::try_from_bytes(pk).expect("Prover key is expected to by valid")
}

fn create_circuit<const I: usize>(
    tx_circuit_vec: TxCircuitVec,
) -> Result<TxCircuit<NOTES_TREE_DEPTH, I>, Error> {
    Ok(TxCircuit {
        input_notes_info: tx_circuit_vec
            .input_notes_info
            .try_into()
            .map_err(|e| Error::PhoenixCircuit(format!("{e:?}")))?,
        output_notes_info: tx_circuit_vec.output_notes_info,
        payload_hash: tx_circuit_vec.payload_hash,
        root: tx_circuit_vec.root,
        deposit: tx_circuit_vec.deposit,
        max_fee: tx_circuit_vec.max_fee,
        sender_pk: tx_circuit_vec.sender_pk,
        signatures: tx_circuit_vec.signatures,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prove_tx_circuit() {
        let tx_circuit_vec_bytes =
            hex::decode(include_str!("../tests/tx_circuit_vec.hex")).unwrap();
        let _proof = LocalProver.prove(&tx_circuit_vec_bytes).unwrap();
    }
}
