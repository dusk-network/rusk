// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::TransferContract;

use dusk_abi::ContractId;
use dusk_bls12_381::BlsScalar;
use phoenix_core::{Crossover, Message};

const VD_STCT: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/b1f318350b747200deaa17172a0552c460fe5997f1dcf87d72186f9952d5af79.vd"));
const VD_STCO: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/3cefe190dd7d4ed0f2dfae8d95f7552e22fc7b073e3441b72269478e5c8d9924.vd"));
const VD_WDFT: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/701e5ad43e8f4afce9e2d714ef0ee88d33fe9404ce0b88733be4d602eadf51e7.vd"));
const VD_WDFO: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/88afddca21d7285681173a1b3f529374cc98574347471d29b2ba7ee4ff624267.vd"));

const VD_EXEC_1_2: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/2253a0e4f0ad48a1829ebe2e5c65724969b18eb64ca02e1b06a0b3679c66e467.vd"));
const VD_EXEC_2_2: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/610f9fe94d561bca6c0e6ae6497fbb16efbe741dd75caf12614b4b20227d8930.vd"));
const VD_EXEC_3_2: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/d954be1a252ade293eaa8067b626e6dbbf5e5b57513eaa10daa67fd098c9e21f.vd"));
const VD_EXEC_4_2: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/9b9df193bb76ba1d8e2936a2f5c907dd36c91775b71a390b4b1356199ac22d30.vd"));

impl TransferContract {
    pub const fn verifier_data_execute(inputs: usize) -> &'static [u8] {
        match inputs {
            1 => VD_EXEC_1_2,
            2 => VD_EXEC_2_2,
            3 => VD_EXEC_3_2,
            4 => VD_EXEC_4_2,
            _ => &[],
        }
    }

    pub const fn verifier_data_stco() -> &'static [u8] {
        VD_STCO
    }

    pub const fn verifier_data_stct() -> &'static [u8] {
        VD_STCT
    }

    pub const fn verifier_data_wdft() -> &'static [u8] {
        VD_WDFT
    }

    pub const fn verifier_data_wdfo() -> &'static [u8] {
        VD_WDFO
    }

    pub fn sign_message_stct(
        crossover: &Crossover,
        value: u64,
        address: &ContractId,
    ) -> BlsScalar {
        let mut m = crossover.to_hash_inputs().to_vec();

        m.push(value.into());
        m.push(rusk_abi::contract_to_scalar(address));

        #[cfg(not(target_arch = "wasm32"))]
        let message = dusk_poseidon::sponge::hash(m.as_slice());

        #[cfg(target_arch = "wasm32")]
        let message = rusk_abi::poseidon_hash(m);

        message
    }

    pub fn sign_message_stco(
        crossover: &Crossover,
        message: &Message,
        address: &ContractId,
    ) -> BlsScalar {
        let mut m = crossover.to_hash_inputs().to_vec();

        m.extend(&message.to_hash_inputs());
        m.push(rusk_abi::contract_to_scalar(address));

        #[cfg(not(target_arch = "wasm32"))]
        let message = dusk_poseidon::sponge::hash(m.as_slice());

        #[cfg(target_arch = "wasm32")]
        let message = rusk_abi::poseidon_hash(m);

        message
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use dusk_plonk::prelude::Circuit;
    use transfer_circuits::*;
    #[test]
    fn circuits_id() {
        // This test is required to explicitly check that circuits ID are the
        // one expected.
        //
        // When a circuit id change, it should be noticed with a compiler error
        // because the circuits key file are renamed. But this error is
        // not raised if the `make keys` command is configured to preserve old
        // keys (like the one launche by the CI)

        test_circuit::<SendToContractTransparentCircuit>(
            "b1f318350b747200deaa17172a0552c460fe5997f1dcf87d72186f9952d5af79",
        );
        test_circuit::<SendToContractObfuscatedCircuit>(
            "3cefe190dd7d4ed0f2dfae8d95f7552e22fc7b073e3441b72269478e5c8d9924",
        );
        test_circuit::<WithdrawFromTransparentCircuit>(
            "701e5ad43e8f4afce9e2d714ef0ee88d33fe9404ce0b88733be4d602eadf51e7",
        );
        test_circuit::<WithdrawFromObfuscatedCircuit>(
            "88afddca21d7285681173a1b3f529374cc98574347471d29b2ba7ee4ff624267",
        );
        test_circuit::<ExecuteCircuitOneTwo>(
            "2253a0e4f0ad48a1829ebe2e5c65724969b18eb64ca02e1b06a0b3679c66e467",
        );
        test_circuit::<ExecuteCircuitTwoTwo>(
            "610f9fe94d561bca6c0e6ae6497fbb16efbe741dd75caf12614b4b20227d8930",
        );
        test_circuit::<ExecuteCircuitThreeTwo>(
            "d954be1a252ade293eaa8067b626e6dbbf5e5b57513eaa10daa67fd098c9e21f",
        );
        test_circuit::<ExecuteCircuitFourTwo>(
            "9b9df193bb76ba1d8e2936a2f5c907dd36c91775b71a390b4b1356199ac22d30",
        );
    }

    fn test_circuit<T>(expected_id: &str)
    where
        T: Circuit,
    {
        let expected_id = hex::decode(expected_id).expect("Cannot decode hex");
        assert_eq!(
            T::CIRCUIT_ID,
            &expected_id[..],
            "Check failed for {} circuit",
            std::any::type_name::<T>()
        );
    }
}
