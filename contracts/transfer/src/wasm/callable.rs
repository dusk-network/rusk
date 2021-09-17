// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::TransferContract;

use alloc::vec::Vec;
use dusk_abi::{ContractId, Transaction};
use dusk_bls12_381::BlsScalar;
use dusk_jubjub::JubJubExtended;
use dusk_pki::{Ownable, StealthAddress};
use phoenix_core::{Crossover, Fee, Message, Note};
use rusk_abi::PaymentInfo;

impl TransferContract {
    pub fn send_to_contract_transparent(
        &mut self,
        address: ContractId,
        value: u64,
        spend_proof: Vec<u8>,
    ) -> bool {
        let (crossover, pk) = self
            .take_crossover()
            .expect("The crossover is mandatory for STCT!");

        let message = Self::sign_message_stct(&crossover, value, &address);

        let mut pi = Vec::with_capacity(6);

        pi.push(crossover.value_commitment().into());
        pi.push(pk.as_ref().into());
        pi.push(message.into());
        pi.push(value.into());

        //  1. v < 2^64
        //  2. B_a↦ = B_a↦ + v
        self.add_balance(address, value)
            .expect("Failed to add the balance to the provided address!");

        //  3. if a.isPayable() ↦ true then continue
        match rusk_abi::payment_info(address) {
            PaymentInfo::Transparent(_) | PaymentInfo::Any(_) => (),
            _ => panic!("The caller doesn't accept transparent notes"),
        }

        //  4. verify(C.c, v, π)
        let vd = Self::verifier_data_stct();
        Self::assert_proof(spend_proof, vd, pi)
            .expect("Failed to verify the provided proof!");

        //  5. C ← C(0,0,0)
        //  Crossover is already taken

        true
    }

    pub fn withdraw_from_transparent(
        &mut self,
        value: u64,
        note: Note,
        spend_proof: Vec<u8>,
    ) -> bool {
        let address = dusk_abi::caller();
        let mut pi = Vec::with_capacity(3);

        pi.push(value.into());
        pi.push(note.value_commitment().into());

        //  1. a ∈ B↦
        //  2. B_a↦ ← B_a↦ − v
        self.sub_balance(&address, value)
            .expect("Failed to subtract the balance from the provided address");

        //  3. N↦.append(N_p^t)
        //  4. N_p^* ← encode(N_p^t)
        //  5. N.append(N_p^*)
        self.push_note_current_height(note)
            .expect("Failed to append the provided note to the state!");

        //  6. verify(C.c, M, pk, π)
        let vd = Self::verifier_data_wdft();
        Self::assert_proof(spend_proof, vd, pi)
            .expect("Failed to verify the provided proof!");

        true
    }

    pub fn send_to_contract_obfuscated(
        &mut self,
        address: ContractId,
        message: Message,
        message_address: StealthAddress,
        spend_proof: Vec<u8>,
    ) -> bool {
        let (crossover, crossover_pk) = self
            .take_crossover()
            .expect("The crossover is mandatory for STCO!");

        let sign_message =
            Self::sign_message_stco(&crossover, &message, &address);

        let (message_psk_a, message_psk_b) =
            match rusk_abi::payment_info(address) {
                PaymentInfo::Obfuscated(Some(k))
                | PaymentInfo::Any(Some(k)) => (*k.A(), *k.B()),

                PaymentInfo::Obfuscated(None) | PaymentInfo::Any(None) => {
                    (JubJubExtended::identity(), JubJubExtended::identity())
                }

                _ => panic!("The caller doesn't accept transparent notes"),
            };

        let mut pi = Vec::with_capacity(12 + message.cipher().len());

        pi.push(crossover.value_commitment().into());
        pi.push(message.value_commitment().into());
        pi.push(message_psk_a.into());
        pi.push(message_psk_b.into());
        pi.push(message_address.pk_r().as_ref().into());
        pi.push(message.nonce().into());
        pi.extend(message.cipher().iter().map(|c| c.into()));
        pi.push(Self::contract_to_scalar(&address).into());
        pi.push(crossover.nonce().into());
        pi.extend(crossover.encrypted_data().cipher().iter().map(|c| c.into()));
        pi.push(sign_message.into());
        pi.push(crossover_pk.as_ref().into());

        //  1. S_a↦.append((pk, R))
        //  2. M_a↦.M_pk↦.append(M)
        self.push_message(address, message_address, message)
            .expect("Failed to append the message to the state!");

        //  3. if a.isPayable() → true, obf, psk_a? then continue
        //  4. verify(C.c, M, pk, π)
        let vd = Self::verifier_data_stco();
        Self::assert_proof(spend_proof, vd, pi)
            .expect("Failed to verify the provided proof!");

        //  5. C←(0,0,0)
        //  Crossover is already taken

        true
    }

    pub fn withdraw_from_obfuscated(
        &mut self,
        message: Message,
        message_address: StealthAddress,
        change: Message,
        change_address: StealthAddress,
        output: Note,
        spend_proof: Vec<u8>,
    ) -> bool {
        let address = dusk_abi::caller();

        let (change_psk_a, change_psk_b) =
            match rusk_abi::payment_info(address) {
                PaymentInfo::Obfuscated(Some(k))
                | PaymentInfo::Any(Some(k)) => (*k.A(), *k.B()),

                PaymentInfo::Obfuscated(None) | PaymentInfo::Any(None) => {
                    (JubJubExtended::identity(), JubJubExtended::identity())
                }

                _ => panic!("The caller doesn't accept transparent notes"),
            };

        let mut pi = Vec::with_capacity(4);

        pi.push(message.value_commitment().into());
        pi.push(change.value_commitment().into());
        pi.push(output.value_commitment().into());
        pi.push(change_psk_a.into());
        pi.push(change_psk_b.into());
        pi.push(change_address.pk_r().as_ref().into());
        pi.push(change.nonce().into());
        pi.extend(change.cipher().iter().map(|c| c.into()));

        //  1. a ∈ M↦
        //  2. pk ∈ M_a↦
        //  3. M_a↦.delete(pk)
        self
            .take_message_from_address_key(&address, message_address.pk_r())
            .expect(
            "Failed to take a message from the provided address/key mapping!",
        );

        self.push_message(address, change_address, change)
            .expect("Failed to append the chanve to the state!");

        //  6. if a.isPayable() → true, obf, psk_a? then continue
        match rusk_abi::payment_info(address) {
            PaymentInfo::Obfuscated(_) | PaymentInfo::Any(_) => (),
            _ => panic!("This contract accepts only obfuscated notes!"),
        }

        self.push_note_current_height(output)
            .expect("Failed to append the provided note to the state!");

        //  7. verify(c, M_c, No.c, π)
        let vd = Self::verifier_data_wdfo();
        Self::assert_proof(spend_proof, vd, pi)
            .expect("Failed to verify the provided proof!");

        true
    }

    pub fn withdraw_from_transparent_to_contract(
        &mut self,
        to: ContractId,
        value: u64,
    ) -> bool {
        let from = dusk_abi::caller();

        //  1. from ∈ B↦
        //  2. B_from↦ ← B_from↦ − v
        self.sub_balance(&from, value).expect(
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
        let crossover_commitment = crossover
            .map(|c| c.value_commitment().clone())
            .unwrap_or_default();
        let inputs = nullifiers.len();
        let outputs = notes.len();

        let mut pi = Vec::with_capacity(5 + inputs + 2 * outputs);

        pi.push(anchor.into());
        pi.extend(nullifiers.iter().map(|n| n.into()));
        pi.push(crossover_commitment.into());
        pi.push(fee.gas_limit.into());
        pi.extend(notes.iter().map(|n| n.value_commitment().into()));

        let tx_hash = Self::tx_hash(pi.as_slice());
        pi.push(tx_hash.into());

        //  1. α ∈ R
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
        if fee.gas_price < minimum_gas_price {
            panic!(
                "The gas price is below the minimum `{:?}`!",
                minimum_gas_price
            );
        }

        // 10. verify(α, ν[], C.c, No.c[], fee)
        let vd = Self::verifier_data_execute(inputs, outputs);
        Self::assert_proof(spend_proof, vd, pi)
            .expect("Failed to verify the provided proof!");

        // 11. if ∣k∣≠0 then call(k)
        self.var_crossover = crossover;
        self.var_crossover_pk
            .replace((*fee.stealth_address().pk_r().as_ref()).into());
        if let Some((contract, tx)) = call {
            let ret = dusk_abi::transact_raw(self, &contract, &tx)
                .expect("Failed to execute the provided call transaction!");

            let _: bool =
                ret.cast().expect("Failed to cast returned value to void!");
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
