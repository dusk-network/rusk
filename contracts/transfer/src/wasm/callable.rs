// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::{internal, keys};
use crate::TransferContract;

use alloc::vec::Vec;
use canonical::Store;
use dusk_abi::{ContractId, Transaction};
use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;
use dusk_jubjub::{
    JubJubAffine, JubJubScalar, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_pki::{Ownable, PublicKey};
use dusk_poseidon::cipher::PoseidonCipher;
use phoenix_core::{Crossover, Fee, Message, Note, NoteType};

impl<S: Store> TransferContract<S> {
    pub fn send_to_contract_transparent(
        &mut self,
        address: BlsScalar,
        value: u64,
        spend_proof: Vec<u8>,
    ) -> bool {
        // Build proof public inputs
        let scalars = 1 + BlsScalar::SIZE;
        let points = (1 + JubJubAffine::SIZE) * 2;

        let mut pi = Vec::with_capacity(scalars + points);
        let label = "transfer-send-to-contract-transparent";

        let (crossover, pk) = self
            .take_crossover()
            .expect("The crossover is mandatory for STCT!");

        internal::extend_pi_jubjub_affine(
            &mut pi,
            &crossover.value_commitment().clone().into(),
        );
        internal::extend_pi_jubjub_affine(&mut pi, &pk.as_ref().clone().into());

        //  1. v < 2^64
        //  2. B_a↦ = B_a↦ + v
        self.add_balance(address, value)
            .expect("Failed to add the balance to the provided address!");

        //  3. if a.isPayable() ↦ true then continue
        Self::assert_payable(&address)
            .expect("The provided address is not payable!");

        //  4. verify(C.c, v, π)
        let vk = keys::stct();
        Self::assert_proof(label, vk, spend_proof, pi)
            .expect("Failed to verify the provided proof!");

        //  5. C ← C(0,0,0)
        //  Crossover is already taken

        true
    }

    pub fn withdraw_from_transparent(
        &mut self,
        address: BlsScalar,
        note: Note,
    ) -> bool {
        let value = match (note.note(), note.value(None)) {
            (NoteType::Transparent, Ok(v)) => v,
            _ => panic!("The provided note must be a transparent note!"),
        };

        //  1. a ∈ B↦
        //  2. B_a↦ ← B_a↦ − v
        self.sub_balance(address, value)
            .expect("Failed to subtract the balance from the provided address");

        //  3. N↦.append(N_p^t)
        //  4. N_p^* ← encode(N_p^t)
        //  5. N.append(N_p^*)
        self.push_note_current_height(note)
            .expect("Failed to append the provided note to the state!");

        true
    }

    pub fn send_to_contract_obfuscated(
        &mut self,
        address: BlsScalar,
        message: Message,
        r: JubJubAffine,
        pk: PublicKey,
        spend_proof: Vec<u8>,
    ) -> bool {
        let scalars =
            (1 + BlsScalar::SIZE) * (1 + PoseidonCipher::cipher_size());
        let points = (1 + JubJubAffine::SIZE) * 4;

        let mut pi = Vec::with_capacity(scalars + points);
        let label = "transfer-send-to-contract-obfuscated";

        let (crossover, crossover_pk) = self
            .take_crossover()
            .expect("The crossover is mandatory for STCT!");

        internal::extend_pi_jubjub_affine(
            &mut pi,
            &crossover.value_commitment().clone().into(),
        );
        internal::extend_pi_jubjub_affine(
            &mut pi,
            &crossover_pk.as_ref().clone().into(),
        );
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
        self.push_message(address, pk, r, message)
            .expect("Failed to append the message to the state!");

        //  3. if a.isPayable() → true, obf, psk_a? then continue
        Self::assert_payable(&address)
            .expect("The provided address is not payable!");

        //  4. verify(C.c, M, pk, π)
        let vk = keys::stco();
        Self::assert_proof(label, vk, spend_proof, pi)
            .expect("Failed to verify the provided proof!");

        //  5. C←(0,0,0)
        //  Crossover is already taken

        true
    }

    pub fn withdraw_from_obfuscated(
        &mut self,
        address: BlsScalar,
        message: Message,
        r: JubJubAffine,
        pk: PublicKey,
        note: Note,
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
        // FIXME Compute the sum of message commitments
        // https://github.com/dusk-network/rusk/issues/192
        let _message = self
            .take_message_from_address_key(&address, &pk)
            .expect(
            "Failed to take a message from the provided address/key mapping!",
        );

        //  4. if |M_c|=1 then S_a↦.append((pk_c, R_c))
        //  5. if |M_c|=1 then M_a↦.M_pk↦.append(M_c)
        self.push_message(address, pk, r, message)
            .expect("Failed to push the provided message to the state!");

        //  6. if a.isPayable() → true, obf, psk_a? then continue
        Self::assert_payable(&address)
            .expect("The provided address is not payable!");

        //  7. verify(c, M_c, No.c, π)
        let vk = keys::wdfo();
        Self::assert_proof(label, vk, spend_proof, pi)
            .expect("Failed to verify the provided proof!");

        // FIXME Non-documented step
        // https://github.com/dusk-network/rusk/issues/192
        self.push_note_current_height(note)
            .expect("Failed to append the provided note to the state!");

        true
    }

    // FIXME Wrong documentation specification
    // The documentation suggests we should threat the withdraw and deposit
    // values differently. Its probably a nit and they should be the same
    // https://github.com/dusk-network/rusk/issues/198
    pub fn withdraw_from_transparent_to_contract(
        &mut self,
        from: BlsScalar,
        to: BlsScalar,
        value: u64,
    ) -> bool {
        //  1. from ∈ B↦
        //  2. B_from↦ ← B_from↦ − v
        self.sub_balance(from, value).expect(
            "Failed to subtract the balance from the provided address!",
        );

        //  3. B_to↦ = B_to↦ + v
        self.add_balance(to, value)
            .expect("Failed to add the balance to the provided address!");

        true
    }

    pub fn execute(
        &mut self,
        anchor: BlsScalar,
        nullifiers: Vec<BlsScalar>,
        fee: Fee,
        crossover: Option<Crossover>,
        notes: Vec<Note>,
        spend_proof: Vec<u8>,
        call: Option<(ContractId, Transaction)>,
    ) -> bool {
        let inputs = nullifiers.len();
        let outputs = notes.len();

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
            &crossover.map(|c| c.value_commitment().into()).unwrap_or(
                ((GENERATOR_EXTENDED * JubJubScalar::zero())
                    + (GENERATOR_NUMS_EXTENDED * JubJubScalar::zero()))
                .into(),
            ),
        );
        notes.iter().for_each(|n| {
            internal::extend_pi_jubjub_affine(
                &mut pi,
                &n.value_commitment().into(),
            )
        });
        // FIXME fetch the tx hash
        // https://github.com/dusk-network/rusk/issues/197
        internal::extend_pi_bls_scalar(&mut pi, &BlsScalar::zero());

        //  1. α ∈ R
        // FIXME Use proper root
        // https://github.com/dusk-network/rusk/issues/224
        let _ = anchor;
        let anchor = BlsScalar::one();
        if !self
            .root_exists(&anchor)
            .expect("Failed to check if the anchor exists!")
        {
            panic!("Anchor not found in the state!");
        }

        //  2. ν[] !∈ Nullifiers
        if self
            .any_nullifier_exists(nullifiers.as_slice())
            .expect("Failed to check if the nullifier already exists!")
        {
            panic!("The provided nullifier already exists!");
        }

        //  3. Nullifiers.append(ν[])
        self.extend_nullifiers(nullifiers)
            .expect("Failed to append the nullifiers to the state!");

        //  4. if |C|=0 then set C ← (0,0,0)
        //  Crossover is received as option

        //  5. N↦.append((No.R[], No.pk[])
        //  6. Notes.append(No[])
        self.extend_notes(notes)
            .expect("Failed to append the notes to the state!");

        //  7. g_l < 2^64
        //  8. g_pmin < g_p
        //  9. fee ← g_l ⋅ g_p
        let minimum_gas_price = Self::minimum_gas_price();
        if fee.gas_price <= minimum_gas_price {
            panic!(
                "The gas price is below the minimum `{:?}`!",
                minimum_gas_price
            );
        }

        // 10. verify(α, ν[], C.c, No.c[], fee)
        let vk = keys::exec(inputs, outputs);
        Self::assert_proof(label, vk, spend_proof, pi)
            .expect("Failed to verify the provided proof!");

        // 11. if ∣k∣≠0 then call(k)
        self.var_crossover = crossover;
        self.var_crossover_pk
            .replace((*fee.stealth_address().pk_r().as_ref()).into());
        if let Some((contract, tx)) = call {
            dusk_abi::debug!("Inter-contract call {:?}", contract);

            let ret = dusk_abi::transact_raw(self, &contract, &tx)
                .expect("Failed to execute the provided call transaction!");

            dusk_abi::debug!("Inter-contract call finished {:?}", contract);

            let ret: bool = ret.cast(S::default())
                .expect("Failed to fetch the return value from the provided call transaction!");

            if !ret {
                panic!("The provided call transaction failed!");
            }
        }

        // 12. if C≠(0,0,0) then N_p^o ← constructObfuscatedNote(C, R, pk)
        // 13. N↦.append((N_p^o.R, N_p^o.pk))
        // 14. Notes.append(N_p^o)
        // 15. N_p^t←constructTransparentNote(g, R, pk)
        // 16. N_p^*←encode(N_p^t)
        // 17. N↦.append((N_p^t.R, N_p^t.pk))
        // 18. Notes.append(N_p^*)
        self.push_fee_crossover(fee)
            .expect("Failed to append the fee and the crossover to the state!");

        self.update_root()
            .expect("Failed to update the state of the tree!");

        true
    }
}
