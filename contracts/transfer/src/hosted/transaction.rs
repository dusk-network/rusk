// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::Contract;

use canonical::Store;
use dusk_bls12_381::BlsScalar;
use dusk_jubjub::{JubJubAffine, JubJubScalar};
use phoenix_core::{Note, NoteType};

// Currently plonk structure definition isn't no-std compatible.
// For the progress, check:
// https://github.com/dusk-network/plonk/tree/vlopes11/no-std
use dusk_plonk::proof_system::proof::Proof;

extern "C" {
    fn verify_proof(
        pub_inputs_len: usize,
        pub_inputs: &u8,
        circuit_crate_len: usize,
        circuit_crate: &u8,
        circuit_label_len: usize,
        circuit_label: &u8,
        proof: &u8,
    ) -> i32;
}

impl<S: Store> Contract<S> {
    pub fn execute(
        &mut self,
        anchor: BlsScalar,
        nullifiers: &[BlsScalar],
        crossover: JubJubAffine,
        notes: &[Note],
        gas_limit: u64,
        gas_price: u64,
        R: JubJubAffine,
        return_pk: JubJubAffine,
        spend_proof: Proof,
        call: u8,
    ) -> bool {
        //  1. g_l < 2^64
        //  Gas limit, as u64 representation, is validated by default

        //  2. g_p > g_pmin
        if gas_price <= Self::minimum_gas_price() {
            return false;
        }

        //  3. α ∈ R
        if !self.root_exists(&anchor).unwrap_or(false) {
            return false;
        }

        //  4. ν[] ∉ N
        if self.any_nullifier_exists(nullifiers).unwrap_or(true) {
            return false;
        }

        //  5. verify(α, ν[], C.c, No.c[], r)
        //  6. N.append(ν[])
        //  7. if ∣C∣ = 0 then set C ← (0,0,0)
        //  8. N↦.append((No.R[], No.pk[]))
        //  9. N.append(No[])
        // 10. r ← g_l ⋅ g_p
        // 11. if ∣k∣ ≠ 0 then call(k)
        // 12. if C ≠ (0,0,0) then No_p ← constructObfuscatedNote(C, R, pk)
        // 13. N↦.append((No_p.R, No_p.pk))
        // 14. N.append(No_p)
        // 15. Nt_p ← constructTransparentNote(g_r, R, pk)
        // 16. N∗_p ← encode(Nt_p)
        // 17. N↦.append((Nt_p.R, Nt_p.pk))
        // 18. N.append(N∗_p)

        true
    }

    pub fn send_to_contract_transparent(
        &mut self,
        address: BlsScalar,
        value: u64,
        spend_proof: Proof,
    ) -> bool {
        // 1. v < 2^{64}
        // This is automatically granted for transparent notes because the value representation
        // is `u64`

        // 2. map[contract address -> value] += v
        let balance = self.get_balance(address);
        if self.balance_mut().insert(address, balance + value).is_err() {
            return false;
        }

        // 3. Validate address.isPayable()
        // TODO, `isPayable` is not defined

        // 4. Verify the crossover commitment, value and proof
        let proof = spend_proof.to_bytes();
        let proof = unsafe {
            verify_proof(
                0,
                &[0u8][0],
                17,
                &b"transfer-circuits"[0],
                37,
                &b"transfer-send-to-contract-transparent"[0],
                &proof[0],
            )
        };
        if proof != 1 {
            return false;
        }

        true
    }
}
