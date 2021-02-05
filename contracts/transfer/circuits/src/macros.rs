// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#[macro_export]
macro_rules! circuit_common_methods {
    ( $s:expr ) => {
        fn get_trim_size(&self) -> usize {
            1 << $s
        }

        fn set_trim_size(&mut self, _size: usize) {
            // N/A, fixed size circuit
        }

        fn get_mut_pi_positions(&mut self) -> &mut Vec<PublicInput> {
            &mut self.pi_positions
        }

        /// Return a reference to the Public Inputs storage of the circuit.
        fn get_pi_positions(&self) -> &Vec<PublicInput> {
            &self.pi_positions
        }
    };
}

#[macro_export]
macro_rules! rusk_profile_methods {
    ( $s:ident, $l:block ) => {
        pub fn rusk_label(&$s) -> String $l

        pub fn rusk_circuit_args(
            &self,
        ) -> Result<(PublicParameters, ProverKey, VerifierKey)> {
            let keys = rusk_profile::keys_for(env!("CARGO_PKG_NAME"));
            let (pk, vk) = keys
                .get(self.rusk_label().as_str())
                .ok_or(anyhow!("Failed to get keys from Rusk profile"))?;

            let pk = ProverKey::from_bytes(pk.as_slice())?;
            let vk = VerifierKey::from_bytes(vk.as_slice())?;

            let pp = rusk_profile::get_common_reference_string().map_err(|e| {
                anyhow!("Failed to fetch CRS from rusk profile: {}", e)
            })?;

            let pp =
                unsafe { PublicParameters::from_slice_unchecked(pp.as_slice())? };

            Ok((pp, pk, vk))
        }
    };
}
