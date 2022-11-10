// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::vec::Vec;

use crate::transfer::TransferState;

use dusk_bls12_381::BlsScalar;
use dusk_jubjub::{JubJubAffine, JubJubExtended};
use dusk_pki::Ownable;
use phoenix_core::Note;
use rusk_abi::{PaymentInfo, RawResult, RawTransaction, State};
use transfer_contract_types::*;

impl TransferState {
    pub fn mint(&mut self, mint: Mint) -> bool {
        // Only the stake contract can mint notes to a particular stealth
        // address. This happens when the reward for staking and participating
        // in the consensus is withdrawn.
        if rusk_abi::caller() != rusk_abi::stake_module() {
            panic!("Can only be called by the stake contract!")
        }

        let note =
            Note::transparent_stealth(mint.address, mint.value, mint.nonce);

        self.push_note_current_height(note);

        true
    }

    pub fn send_to_contract_transparent(&mut self, stct: Stct) -> bool {
        let (crossover, pk) = self
            .take_crossover()
            .expect("The crossover is mandatory for STCT!");

        let message =
            Self::sign_message_stct(&crossover, stct.value, &stct.module);

        let mut pi = Vec::with_capacity(6);

        pi.push(crossover.value_commitment().into());
        pi.push(stct.value.into());
        pi.push(pk.as_ref().into());
        pi.push(message.into());

        //  1. v < 2^64
        //  2. B_a↦ = B_a↦ + v
        self.add_balance(stct.module, stct.value);

        //  3. if a.isPayable() ↦ true then continue
        match rusk_abi::payment_info(stct.module) {
            PaymentInfo::Transparent(_) | PaymentInfo::Any(_) => (),
            _ => panic!("The caller doesn't accept transparent notes"),
        }

        //  4. verify(C.c, v, π)
        let vd = Self::verifier_data_stct();
        Self::assert_proof(vd, stct.proof, pi)
            .expect("Failed to verify the provided proof!");

        //  5. C ← C(0,0,0)
        //  Crossover is already taken

        true
    }

    pub fn withdraw_from_contract_transparent(&mut self, wfct: Wfct) -> bool {
        let address = rusk_abi::caller();
        let mut pi = Vec::with_capacity(3);

        pi.push(wfct.value.into());
        pi.push(wfct.note.value_commitment().into());

        //  1. a ∈ B↦
        //  2. B_a↦ ← B_a↦ − v
        self.sub_balance(&address, wfct.value)
            .expect("Failed to subtract the balance from the provided address");

        //  3. N↦.append(N_p^t)
        //  4. N_p^* ← encode(N_p^t)
        //  5. N.append(N_p^*)
        self.push_note_current_height(wfct.note);

        //  6. verify(C.c, M, pk, π)
        let vd = Self::verifier_data_wdft();
        Self::assert_proof(vd, wfct.proof, pi)
            .expect("Failed to verify the provided proof!");

        true
    }

    pub fn send_to_contract_obfuscated(&mut self, stco: Stco) -> bool {
        let (crossover, crossover_pk) = self
            .take_crossover()
            .expect("The crossover is mandatory for STCO!");

        let sign_message =
            Self::sign_message_stco(&crossover, &stco.message, &stco.module);

        let (message_psk_a, message_psk_b) =
            match rusk_abi::payment_info(stco.module) {
                PaymentInfo::Obfuscated(Some(k))
                | PaymentInfo::Any(Some(k)) => (*k.A(), *k.B()),

                PaymentInfo::Obfuscated(None) | PaymentInfo::Any(None) => {
                    (JubJubExtended::identity(), JubJubExtended::identity())
                }

                _ => panic!("The caller doesn't accept transparent notes"),
            };

        let mut pi = Vec::with_capacity(12 + stco.message.cipher().len());

        pi.push(crossover.value_commitment().into());
        pi.push(crossover.nonce().into());
        pi.extend(crossover.encrypted_data().cipher().iter().map(|c| c.into()));
        pi.push(stco.message.value_commitment().into());
        pi.push(message_psk_a.into());
        pi.push(message_psk_b.into());
        pi.push(stco.message_address.pk_r().as_ref().into());
        pi.push(stco.message.nonce().into());
        pi.extend(stco.message.cipher().iter().map(|c| c.into()));
        pi.push(rusk_abi::module_to_scalar(&stco.module).into());
        pi.push(sign_message.into());
        pi.push(crossover_pk.as_ref().into());

        //  1. S_a↦.append((pk, R))
        //  2. M_a↦.M_pk↦.append(M)
        self.push_message(stco.module, stco.message_address, stco.message);

        //  3. if a.isPayable() → true, obf, psk_a? then continue
        //  4. verify(C.c, M, pk, π)
        let vd = Self::verifier_data_stco();
        Self::assert_proof(vd, stco.proof, pi)
            .expect("Failed to verify the provided proof!");

        //  5. C←(0,0,0)
        //  Crossover is already taken

        true
    }

    pub fn withdraw_from_contract_obfuscated(&mut self, wfco: Wfco) -> bool {
        let address = rusk_abi::caller();

        let (change_psk_a, change_psk_b) =
            match rusk_abi::payment_info(address) {
                PaymentInfo::Obfuscated(Some(k))
                | PaymentInfo::Any(Some(k)) => (*k.A(), *k.B()),

                PaymentInfo::Obfuscated(None) | PaymentInfo::Any(None) => {
                    (JubJubExtended::identity(), JubJubExtended::identity())
                }

                _ => panic!("The caller doesn't accept obfuscated notes"),
            };

        let mut pi = Vec::with_capacity(4);

        pi.push(wfco.message.value_commitment().into());
        pi.push(wfco.change.value_commitment().into());
        pi.push(change_psk_a.into());
        pi.push(change_psk_b.into());
        pi.push(wfco.change_address.pk_r().as_ref().into());
        pi.push(wfco.change.nonce().into());
        pi.extend(wfco.change.cipher().iter().map(|c| c.into()));
        pi.push(wfco.output.value_commitment().into());

        //  1. a ∈ M↦
        //  2. pk ∈ M_a↦
        //  3. M_a↦.delete(pk)
        self.take_message_from_address_key(
            &address,
            wfco.message_address.pk_r(),
        )
        .expect(
            "Failed to take a message from the provided address/key mapping!",
        );

        self.push_message(address, wfco.change_address, wfco.change);

        //  6. if a.isPayable() → true, obf, psk_a? then continue
        match rusk_abi::payment_info(address) {
            PaymentInfo::Obfuscated(_) | PaymentInfo::Any(_) => (),
            _ => panic!("This contract accepts only obfuscated notes!"),
        }

        self.push_note_current_height(wfco.output);

        //  7. verify(c, M_c, No.c, π)
        let vd = Self::verifier_data_wdfo();
        Self::assert_proof(vd, wfco.proof, pi)
            .expect("Failed to verify the provided proof!");

        true
    }

    pub fn withdraw_from_contract_transparent_to_contract(
        &mut self,
        wfctc: Wfctc,
    ) -> bool {
        let from = rusk_abi::caller();

        //  1. from ∈ B↦
        //  2. B_from↦ ← B_from↦ − v
        self.sub_balance(&from, wfctc.value).expect(
            "Failed to subtract the balance from the provided address!",
        );

        //  3. B_to↦ = B_to↦ + v
        self.add_balance(wfctc.module, wfctc.value);

        true
    }

    pub fn execute(
        self: &mut State<Self>,
        tx: Transaction,
    ) -> Option<RawResult> {
        // Constant for a pedersen commitment with zero value.
        //
        // Calculated as `G^0 · G'^0`
        pub const ZERO_COMMITMENT: JubJubAffine =
            JubJubAffine::from_raw_unchecked(
                BlsScalar::zero(),
                BlsScalar::one(),
            );

        let crossover_commitment = tx
            .crossover
            .map(|c| c.value_commitment().clone())
            .unwrap_or_default();
        let inputs = tx.nullifiers.len();
        let outputs = tx.outputs.len();

        let hash_bytes = tx.hash_bytes();
        let tx_hash = rusk_abi::hash(hash_bytes);

        let mut pi = Vec::with_capacity(5 + inputs + 2 * outputs);

        pi.push(tx_hash.into());
        pi.push(tx.anchor.into());
        pi.extend(tx.nullifiers.iter().map(|n| n.into()));
        pi.push(crossover_commitment.into());

        let fee_value = tx.fee.gas_limit * tx.fee.gas_price;

        pi.push(fee_value.into());
        pi.extend(tx.outputs.iter().map(|n| n.value_commitment().into()));
        pi.extend(
            (0usize..2usize.saturating_sub(tx.outputs.len()))
                .map(|_| ZERO_COMMITMENT.into()),
        );

        //  1. α ∈ R
        if !self.root_exists(&tx.anchor) {
            panic!("Anchor not found in the state!");
        }

        //  2. ν[] !∈ Nullifiers
        if self.any_nullifier_exists(tx.nullifiers.as_slice()) {
            panic!("A provided nullifier already exists!");
        }

        //  3. Nullifiers.append(ν[])
        self.extend_nullifiers(tx.nullifiers);

        //  4. if |C|=0 then set C ← (0,0,0)
        //  Crossover is received as option

        //  5. N↦.append((No.R[], No.pk[])
        //  6. Notes.append(No[])
        self.extend_notes(tx.outputs);

        //  7. g_l < 2^64
        //  8. g_pmin < g_p
        //  9. fee ← g_l ⋅ g_p
        let minimum_gas_price = Self::minimum_gas_price();
        if tx.fee.gas_price < minimum_gas_price {
            panic!(
                "The gas price is below the minimum `{:?}`!",
                minimum_gas_price
            );
        }

        // 10. verify(α, ν[], C.c, No.c[], fee)
        let vd = Self::verifier_data_execute(inputs);
        Self::assert_proof(vd, tx.proof, pi)
            .expect("Failed to verify the provided proof!");

        // 11. if ∣k∣≠0 then call(k)
        self.var_crossover = tx.crossover;
        self.var_crossover_pk
            .replace((*tx.fee.stealth_address().pk_r().as_ref()).into());

        let res = tx.call.map(|(module, fn_name, data)| {
            let raw_tx = RawTransaction::new(&fn_name, data);
            self.transact_raw(module, raw_tx)
        });

        // 12. if C≠(0,0,0) then N_p^o ← constructObfuscatedNote(C, R, pk)
        // 13. N↦.append((N_p^o.R, N_p^o.pk))
        // 14. Notes.append(N_p^o)
        // 15. N_p^t←constructTransparentNote(g, R, pk)
        // 16. N_p^*←encode(N_p^t)
        // 17. N↦.append((N_p^t.R, N_p^t.pk))
        // 18. Notes.append(N_p^*)
        self.push_fee_crossover(tx.fee)
            .expect("Failed to append the fee and the crossover to the state!");

        self.update_root();

        res
    }
}
