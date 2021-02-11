// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::Tree;

use alloc::vec::Vec;
use canonical::{Canon, Store};
use canonical_derive::Canon;
use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;
use dusk_jubjub::JubJubAffine;
use dusk_kelvin_map::Map;
use dusk_pki::PublicKey;
use dusk_poseidon::cipher::PoseidonCipher;
use phoenix_core::{Crossover, Fee, Message, Note, NoteType};

mod call;
mod internal;

use internal::PublicKeyBytes;

pub use call::Call;

#[derive(Debug, Default, Clone, Canon)]
pub struct Transfer<S: Store> {
    notes: Tree<S>,
    notes_mapping: Map<u64, Vec<Note>, S>,
    nullifiers: Map<BlsScalar, (), S>,
    roots: Map<BlsScalar, (), S>,
    balance: Map<BlsScalar, u64, S>,
    message_mapping: Map<BlsScalar, Map<PublicKeyBytes, Message, S>, S>,
    message_mapping_set: Map<BlsScalar, (PublicKey, JubJubAffine), S>,
}

impl<S: Store> Transfer<S> {
    pub(crate) fn send_to_contract_transparent(
        &mut self,
        address: BlsScalar,
        value: u64,
        value_commitment: JubJubAffine,
        pk: JubJubAffine,
        spend_proof: Vec<u8>,
    ) -> bool {
        // Build proof public inputs
        let scalars = 1 + BlsScalar::SIZE;
        let points = (1 + JubJubAffine::SIZE) * 2;

        let mut pi = Vec::with_capacity(scalars + points);
        let label = "transfer-send-to-contract-transparent";
        internal::extend_pi_jubjub_affine(&mut pi, &value_commitment);
        internal::extend_pi_jubjub_affine(&mut pi, &pk);
        internal::extend_pi_bls_scalar(&mut pi, &BlsScalar::from(value));

        //  1. v < 2^64
        //  2. B_a↦ = B_a↦ + v
        if self.add_balance(address, value).is_err() {
            return false;
        }

        //  3. if a.isPayable() ↦ true then continue
        //  TODO

        //  4. verify(C.c, v, π)
        // TODO
        let (_, _, _) = (pi, label, spend_proof);

        //  5. C ← C(0,0,0)
        //  TODO

        true
    }

    pub(crate) fn withdraw_from_transparent(
        &mut self,
        address: BlsScalar,
        note: Note,
    ) -> bool {
        let value = match (note.note(), note.value(None)) {
            (NoteType::Transparent, Ok(v)) => v,
            _ => return false,
        };

        //  1. a ∈ B↦
        //  2. B_a↦ ← B_a↦ − v
        if self.sub_balance(address, value).is_err() {
            return false;
        }

        //  3. N↦.append(N_p^t)
        //  4. N_p^* ← encode(N_p^t)
        //  5. N.append(N_p^*)
        if self.push_note(note).is_err() {
            return false;
        }

        true
    }

    pub(crate) fn send_to_contract_obfuscated(
        &mut self,
        address: BlsScalar,
        message: Message,
        r: JubJubAffine,
        pk: PublicKey,
        crossover_commitment: JubJubAffine,
        crossover_pk: JubJubAffine,
        spend_proof: Vec<u8>,
    ) -> bool {
        let scalars =
            (1 + BlsScalar::SIZE) * (1 + PoseidonCipher::cipher_size());
        let points = (1 + JubJubAffine::SIZE) * 4;

        let mut pi = Vec::with_capacity(scalars + points);
        let label = "transfer-send-to-contract-obfuscated";
        internal::extend_pi_jubjub_affine(&mut pi, &crossover_commitment);
        internal::extend_pi_jubjub_affine(&mut pi, &crossover_pk);
        internal::extend_pi_jubjub_affine(
            &mut pi,
            &message.value_commitment().into(),
        );
        internal::extend_pi_jubjub_scalar(&mut pi, &message.nonce());
        internal::extend_pi_jubjub_affine(&mut pi, &pk.as_ref().into());
        message
            .cipher()
            .iter()
            .for_each(|c| internal::extend_pi_bls_scalar(&mut pi, c));

        //  1. S_a↦.append((pk, R))
        //  2. M_a↦.M_pk↦.append(M)
        if self.push_message(address, pk, r, message).is_err() {
            return false;
        }

        //  3. if a.isPayable() → true, obf, psk_a? then continue
        //  TODO

        //  4. verify(C.c, M, pk, π)
        //  TODO
        let (_, _, _) = (pi, label, spend_proof);

        //  5. C←(0,0,0)
        //  TODO

        true
    }

    pub(crate) fn withdraw_from_obfuscated(
        &mut self,
        address: BlsScalar,
        message: Message,
        r: JubJubAffine,
        pk: PublicKey,
        note: Note, // FIXME nothing is done with this note
        input_value_commitment: JubJubAffine,
        spend_proof: Vec<u8>,
    ) -> bool {
        let scalars =
            (1 + BlsScalar::SIZE) * (1 + PoseidonCipher::cipher_size());
        let points = (1 + JubJubAffine::SIZE) * 4;

        let mut pi = Vec::with_capacity(scalars + points);
        let label = "transfer-withdraw-from-obfuscated";
        internal::extend_pi_jubjub_affine(&mut pi, &input_value_commitment);
        internal::extend_pi_jubjub_affine(
            &mut pi,
            &message.value_commitment().into(),
        );
        internal::extend_pi_jubjub_scalar(&mut pi, &message.nonce());
        internal::extend_pi_jubjub_affine(&mut pi, &pk.as_ref().into());
        message
            .cipher()
            .iter()
            .for_each(|c| internal::extend_pi_bls_scalar(&mut pi, c));
        internal::extend_pi_jubjub_affine(
            &mut pi,
            &note.value_commitment().into(),
        );

        //  1. a ∈ M↦
        //  2. pk ∈ M_a↦
        //  3. M_a↦.delete(pk)
        // FIXME This message is taken and nothing is verified with it
        let _message = match self.take_message_from_address_key(&address, &pk) {
            Ok(m) => m,
            Err(_) => return false,
        };

        //  4. if |M_c|=1 then S_a↦.append((pk_c, R_c))
        //  5. if |M_c|=1 then M_a↦.M_pk↦.append(M_c)
        if self.push_message(address, pk, r, message).is_err() {
            return false;
        }

        //  6. if a.isPayable() → true, obf, psk_a? then continue
        //  TODO

        //  7. verify(c, M_c, No.c, π)
        //  TODO
        let (_, _, _) = (pi, label, spend_proof);

        true
    }

    pub fn execute(
        &mut self,
        anchor: BlsScalar,
        nullifiers: Vec<BlsScalar>,
        crossover: Crossover,
        notes: Vec<Note>,
        fee: Fee,
        spend_proof: Vec<u8>,
        call: Call,
    ) -> bool {
        // Build proof public inputs
        let scalars = (1 + BlsScalar::SIZE) * (3 + nullifiers.len());
        let points = (1 + JubJubAffine::SIZE) * (1 + notes.len());

        let mut pi = Vec::with_capacity(scalars + points);
        let label = Self::rusk_label(nullifiers.len(), notes.len());
        internal::extend_pi_bls_scalar(&mut pi, &anchor);
        nullifiers
            .iter()
            .for_each(|n| internal::extend_pi_bls_scalar(&mut pi, n));
        internal::extend_pi_bls_scalar(
            &mut pi,
            &BlsScalar::from(fee.gas_limit),
        );
        internal::extend_pi_jubjub_affine(
            &mut pi,
            &crossover.value_commitment().into(),
        );
        notes.iter().for_each(|n| {
            internal::extend_pi_jubjub_affine(
                &mut pi,
                &n.value_commitment().into(),
            )
        });
        // FIXME fetch the tx hash
        internal::extend_pi_bls_scalar(&mut pi, &BlsScalar::zero());

        //  1. α ∈ R
        if !self.root_exists(&anchor).unwrap_or(false) {
            return false;
        }

        //  2. ν[] !∈ Nullifiers
        if self
            .any_nullifier_exists(nullifiers.as_slice())
            .unwrap_or(true)
        {
            return false;
        }

        //  3. Nullifiers.append(ν[])
        if self.extend_nullifiers(nullifiers).is_err() {
            return false;
        }

        //  4. if |C|=0 then set C ← (0,0,0)
        //  TODO

        //  5. N↦.append((No.R[], No.pk[])
        //  6. Notes.append(No[])
        if self.extend_notes(notes).is_err() {
            return false;
        }

        //  7. g_l < 2^64
        //  8. g_pmin < g_p
        //  9. fee ← g_l ⋅ g_p
        if fee.gas_price <= Self::minimum_gas_price() {
            return false;
        }

        // 10. verify(α, ν[], C.c, No.c[], fee)
        // TODO
        let (_, _, _) = (pi, label, spend_proof);

        // 11. if ∣k∣≠0 then call(k)
        if !self.internal_call(call) {
            return false;
        }

        // 12. if C≠(0,0,0) then N_p^o ← constructObfuscatedNote(C, R, pk)
        // 13. N↦.append((N_p^o.R, N_p^o.pk))
        // 14. Notes.append(N_p^o)
        // 15. N_p^t←constructTransparentNote(g, R, pk)
        // 16. N_p^*←encode(N_p^t)
        // 17. N↦.append((N_p^t.R, N_p^t.pk))
        // 18. Notes.append(N_p^*)
        if self.push_fee_crossover(fee, crossover).is_err() {
            return false;
        }

        true
    }
}
