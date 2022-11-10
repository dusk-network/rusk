// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::TransferState;

use dusk_bls12_381::BlsScalar;
use phoenix_core::{Crossover, Message};
use rusk_abi::ModuleId;

const VD_STCT: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/d2efc2eb1caaa12ce0877ad248293dbffdbab41e659ca9911f032f344ab77263.vd"));
const VD_STCO: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/cad2cfab7ed15338ac22179c6b8e3351c7ce7320e3f87a559716e57bc2bdda47.vd"));
const VD_WDFT: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/dcc4561c1bbd8a10cd14c9e826d51373567dd41bb2cfd498f92230abc602ed47.vd"));
const VD_WDFO: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/8f7301b53f3af3eb14563c7e474a539a6e12c1248e1e9bdb4b07eeb2ef1a8f2e.vd"));

const VD_EXEC_1_2: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/19c9391f2f03a5206caac2618b8ab32847b6a1e19500fec27a3a96b9a84b200c.vd"));
const VD_EXEC_2_2: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/ea59814e99b4c8789cff85d6623749f823c56383e300761537b3e248c537a033.vd"));
const VD_EXEC_3_2: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/4e03eb1686949f9f17d13d285a4a9c5bc9596a84765f36a3491a981a29135987.vd"));
const VD_EXEC_4_2: &[u8] = include_bytes!(concat!(env!("RUSK_PROFILE_PATH"), "/.rusk/keys/2a34871c45dd993c6217199c5c000aff24621f5953aca3a1755fe052a8e4e7b9.vd"));

impl TransferState {
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
        address: &ModuleId,
    ) -> BlsScalar {
        let mut m = crossover.to_hash_inputs().to_vec();

        m.push(value.into());
        m.push(rusk_abi::module_to_scalar(address));

        #[cfg(not(target_arch = "wasm32"))]
        let message = dusk_poseidon::sponge::hash(m.as_slice());

        #[cfg(target_arch = "wasm32")]
        let message = rusk_abi::poseidon_hash(m);

        message
    }

    pub fn sign_message_stco(
        crossover: &Crossover,
        message: &Message,
        address: &ModuleId,
    ) -> BlsScalar {
        let mut m = crossover.to_hash_inputs().to_vec();

        m.extend(&message.to_hash_inputs());
        m.push(rusk_abi::module_to_scalar(address));

        #[cfg(not(target_arch = "wasm32"))]
        let message = dusk_poseidon::sponge::hash(m.as_slice());

        #[cfg(target_arch = "wasm32")]
        let message = rusk_abi::poseidon_hash(m);

        message
    }
}

#[cfg(test)]
mod tests {
    use transfer_circuits::*;

    macro_rules! test_circuit {
        ($circuit:ty, $id:literal,) => {
            let id = hex::encode(<$circuit>::circuit_id());
            assert_eq!(id, $id, "The circuit IDs should be as expected");
        };
    }

    #[test]
    fn circuits_id() {
        // This test is required to explicitly check that circuits IDs are the
        // ones expected.
        //
        // When a circuit id changes, it should be noticed with a compiler error
        // because the circuits' key file is renamed. This error is not raised
        // if the `make keys` command is configured to preserve old keys (like
        // the one launched by the CI)
        test_circuit!(
            SendToContractTransparentCircuit,
            "d2efc2eb1caaa12ce0877ad248293dbffdbab41e659ca9911f032f344ab77263",
        );
        test_circuit!(
            SendToContractObfuscatedCircuit,
            "cad2cfab7ed15338ac22179c6b8e3351c7ce7320e3f87a559716e57bc2bdda47",
        );
        test_circuit!(
            WithdrawFromTransparentCircuit,
            "dcc4561c1bbd8a10cd14c9e826d51373567dd41bb2cfd498f92230abc602ed47",
        );
        test_circuit!(
            WithdrawFromObfuscatedCircuit,
            "8f7301b53f3af3eb14563c7e474a539a6e12c1248e1e9bdb4b07eeb2ef1a8f2e",
        );
        test_circuit!(
            ExecuteCircuitOneTwo,
            "19c9391f2f03a5206caac2618b8ab32847b6a1e19500fec27a3a96b9a84b200c",
        );
        test_circuit!(
            ExecuteCircuitTwoTwo,
            "ea59814e99b4c8789cff85d6623749f823c56383e300761537b3e248c537a033",
        );
        test_circuit!(
            ExecuteCircuitThreeTwo,
            "4e03eb1686949f9f17d13d285a4a9c5bc9596a84765f36a3491a981a29135987",
        );
        test_circuit!(
            ExecuteCircuitFourTwo,
            "2a34871c45dd993c6217199c5c000aff24621f5953aca3a1755fe052a8e4e7b9",
        );
    }
}
