// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::{internal, keys};
use crate::{
    InternalCall, InternalCallResult, TransferContract, TransferExecute,
};

use alloc::vec::Vec;
use canonical::Store;
use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;
use dusk_jubjub::{
    JubJubAffine, JubJubScalar, GENERATOR_EXTENDED, GENERATOR_NUMS_EXTENDED,
};
use dusk_pki::PublicKey;
use dusk_poseidon::cipher::PoseidonCipher;
use phoenix_core::{Crossover, Message, Note, NoteType};

impl<S: Store> TransferContract<S> {
    fn call(&mut self, call: InternalCall) -> InternalCallResult {
        match call {
            InternalCall::External {
                contract,
                transaction,
                crossover,
            } => {
                let ret = match dusk_abi::transact_raw(&contract, &transaction)
                {
                    Ok(r) => r,
                    Err(_) => return InternalCallResult::error(),
                };

                // FIXME Unnecessary ownership of `Store`
                // https://github.com/dusk-network/rusk-vm/issues/159
                let store: S = unsafe { core::mem::zeroed() };

                // FIXME Assuming the called contract will return only the
                // boolean result https://github.com/dusk-network/rusk/issues/204
                match ret.cast::<bool, _>(store) {
                    Ok(r) if r => InternalCallResult::success(crossover),

                    _ => InternalCallResult::error(),
                }
            }

            InternalCall::None(crossover) => {
                InternalCallResult::success(crossover)
            }

            InternalCall::SendToContractTransparent {
                address,
                value,
                crossover,
                pk,
                spend_proof,
            } => self.send_to_contract_transparent(
                address,
                value,
                crossover,
                pk,
                spend_proof,
            ),

            InternalCall::WithdrawFromTransparent { address, note } => {
                self.withdraw_from_transparent(address, note)
            }

            InternalCall::SendToContractObfuscated {
                address,
                message,
                r,
                pk,
                crossover,
                crossover_pk,
                spend_proof,
            } => self.send_to_contract_obfuscated(
                address,
                message,
                r,
                pk,
                crossover,
                crossover_pk,
                spend_proof,
            ),

            InternalCall::WithdrawFromObfuscated {
                address,
                message,
                r,
                pk,
                note,
                input_value_commitment,
                spend_proof,
            } => self.withdraw_from_obfuscated(
                address,
                message,
                r,
                pk,
                note,
                input_value_commitment,
                spend_proof,
            ),

            InternalCall::WithdrawFromTransparentToContract {
                from,
                to,
                value,
            } => self.withdraw_from_transparent_to_contract(from, to, value),
        }
    }

    fn send_to_contract_transparent(
        &mut self,
        address: BlsScalar,
        value: u64,
        crossover: Crossover,
        pk: PublicKey,
        spend_proof: Vec<u8>,
    ) -> InternalCallResult {
        // Build proof public inputs
        let scalars = 1 + BlsScalar::SIZE;
        let points = (1 + JubJubAffine::SIZE) * 2;

        let mut pi = Vec::with_capacity(scalars + points);
        let label = "transfer-send-to-contract-transparent";
        internal::extend_pi_jubjub_affine(
            &mut pi,
            &crossover.value_commitment().clone().into(),
        );
        internal::extend_pi_jubjub_affine(&mut pi, &pk.as_ref().clone().into());
        internal::extend_pi_bls_scalar(&mut pi, &BlsScalar::from(value));

        //  1. v < 2^64
        //  2. B_a↦ = B_a↦ + v
        if self.add_balance(address, value).is_err() {
            return InternalCallResult::error();
        }

        //  3. if a.isPayable() ↦ true then continue
        //  TODO Use isPayable definition
        //  https://github.com/dusk-network/rusk-vm/issues/151

        //  4. verify(C.c, v, π)
        //  TODO implement proof verification
        //  https://github.com/dusk-network/rusk/issues/194
        let vk = keys::stct();
        let (_, _, _, _) = (pi, label, spend_proof, vk);

        //  5. C ← C(0,0,0)
        InternalCallResult::success(None)
    }

    fn withdraw_from_transparent(
        &mut self,
        address: BlsScalar,
        note: Note,
    ) -> InternalCallResult {
        let value = match (note.note(), note.value(None)) {
            (NoteType::Transparent, Ok(v)) => v,
            _ => return InternalCallResult::error(),
        };

        //  1. a ∈ B↦
        //  2. B_a↦ ← B_a↦ − v
        if self.sub_balance(address, value).is_err() {
            return InternalCallResult::error();
        }

        //  3. N↦.append(N_p^t)
        //  4. N_p^* ← encode(N_p^t)
        //  5. N.append(N_p^*)
        if self.push_note(note).is_err() {
            return InternalCallResult::error();
        }

        InternalCallResult::success(None)
    }

    fn send_to_contract_obfuscated(
        &mut self,
        address: BlsScalar,
        message: Message,
        r: JubJubAffine,
        pk: PublicKey,
        crossover: Crossover,
        crossover_pk: PublicKey,
        spend_proof: Vec<u8>,
    ) -> InternalCallResult {
        let scalars =
            (1 + BlsScalar::SIZE) * (1 + PoseidonCipher::cipher_size());
        let points = (1 + JubJubAffine::SIZE) * 4;

        let mut pi = Vec::with_capacity(scalars + points);
        let label = "transfer-send-to-contract-obfuscated";
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
        if self.push_message(address, pk, r, message).is_err() {
            return InternalCallResult::error();
        }

        //  3. if a.isPayable() → true, obf, psk_a? then continue
        //  TODO Use isPayable definition
        //  https://github.com/dusk-network/rusk-vm/issues/151

        //  4. verify(C.c, M, pk, π)
        //  TODO implement proof verification
        //  https://github.com/dusk-network/rusk/issues/194
        let vk = keys::stco();
        let (_, _, _, _) = (pi, label, spend_proof, vk);

        //  5. C←(0,0,0)
        InternalCallResult::success(None)
    }

    // FIXME nothing is done with the passed note
    // https://github.com/dusk-network/rusk/issues/192
    fn withdraw_from_obfuscated(
        &mut self,
        address: BlsScalar,
        message: Message,
        r: JubJubAffine,
        pk: PublicKey,
        note: Note,
        input_value_commitment: JubJubAffine,
        spend_proof: Vec<u8>,
    ) -> InternalCallResult {
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
        // https://github.com/dusk-network/rusk/issues/192
        let _message = match self.take_message_from_address_key(&address, &pk) {
            Ok(m) => m,
            Err(_) => return InternalCallResult::error(),
        };

        //  4. if |M_c|=1 then S_a↦.append((pk_c, R_c))
        //  5. if |M_c|=1 then M_a↦.M_pk↦.append(M_c)
        if self.push_message(address, pk, r, message).is_err() {
            return InternalCallResult::error();
        }

        //  6. if a.isPayable() → true, obf, psk_a? then continue
        //  TODO Use isPayable definition
        //  https://github.com/dusk-network/rusk-vm/issues/151

        //  7. verify(c, M_c, No.c, π)
        //  TODO implement proof verification
        //  https://github.com/dusk-network/rusk/issues/194
        let vk = keys::wdfo();
        let (_, _, _, _) = (pi, label, spend_proof, vk);

        InternalCallResult::success(None)
    }

    // FIXME Wrong documentation specification
    // https://github.com/dusk-network/rusk/issues/198
    fn withdraw_from_transparent_to_contract(
        &mut self,
        from: BlsScalar,
        to: BlsScalar,
        value: u64,
    ) -> InternalCallResult {
        //  1. from ∈ B↦
        //  2. B_from↦ ← B_from↦ − v
        if self.sub_balance(from, value).is_err() {
            return InternalCallResult::error();
        }

        //  3. B_to↦ = B_to↦ + v
        if self.add_balance(to, value).is_err() {
            return InternalCallResult::error();
        }

        InternalCallResult::success(None)
    }

    pub fn execute(&mut self, call: TransferExecute) -> bool {
        let internal_call = match call.clone().into_internal::<S>() {
            Ok(c) => c,
            Err(_) => return false,
        };
        let TransferExecute {
            anchor,
            nullifiers,
            crossover,
            notes,
            fee,
            spend_proof,
            ..
        } = call;

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
        //  Crossover is received as option

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
        //  TODO implement proof verification
        //  https://github.com/dusk-network/rusk/issues/194
        let vk = keys::exec(inputs, outputs);
        let (_, _, _, _) = (pi, label, spend_proof, vk);

        // 11. if ∣k∣≠0 then call(k)
        let call_result = self.call(internal_call);
        if !call_result.is_success() {
            return false;
        }

        // 12. if C≠(0,0,0) then N_p^o ← constructObfuscatedNote(C, R, pk)
        // 13. N↦.append((N_p^o.R, N_p^o.pk))
        // 14. Notes.append(N_p^o)
        // 15. N_p^t←constructTransparentNote(g, R, pk)
        // 16. N_p^*←encode(N_p^t)
        // 17. N↦.append((N_p^t.R, N_p^t.pk))
        // 18. Notes.append(N_p^*)
        if self.push_fee_crossover(fee, call_result.crossover).is_err() {
            return false;
        }

        if self.update_root().is_err() {
            return false;
        }

        true
    }
}
