// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::alloc::string::ToString;
use crate::circuits::*;
use crate::error::Error;

use alloc::vec::Vec;

use dusk_bls12_381::BlsScalar;
use dusk_bytes::DeserializableSlice;
use dusk_jubjub::{JubJubAffine, JubJubExtended};
use dusk_pki::{Ownable, PublicKey, StealthAddress};
use phoenix_core::transaction::*;
use phoenix_core::{Crossover, Fee, Message, Note};
use poseidon_merkle::Opening as PoseidonOpening;
use rusk_abi::{
    ContractError, ContractId, PaymentInfo, PublicInput, STAKE_CONTRACT,
    TRANSFER_CONTRACT, TRANSFER_DATA_CONTRACT, TRANSFER_LOGIC_CONTRACT,
};
use transfer_contract_types::{Mint, Stct, Wfco, WfcoRaw, Wfct, Wfctc};

/// Arity of the transfer tree.
pub const A: usize = 4;

pub struct TransferOps;

impl TransferOps {
    fn is_transfer_caller() -> bool {
        let transfer_owner = rusk_abi::owner_raw(TRANSFER_CONTRACT).unwrap();
        let caller_id = rusk_abi::caller();
        matches!(rusk_abi::owner_raw(caller_id), Some(caller_owner) if caller_owner.eq(&transfer_owner))
    }

    pub fn mint(&mut self, mint: Mint) -> bool {
        // Only the stake and transfer contracts can mint notes to a particular
        // stealth address. This happens when the reward for staking and
        // participating in the consensus is withdrawn.
        if rusk_abi::caller() != STAKE_CONTRACT && !Self::is_transfer_caller() {
            panic!("Can only be called by the stake and transfer contracts!")
        }

        let note =
            Note::transparent_stealth(mint.address, mint.value, mint.nonce);

        self.push_note_current_height(note);

        true
    }

    pub fn send_to_contract_transparent(&mut self, stct: Stct) -> bool {
        let (crossover, stealth_addr) =
            self.take_crossover().expect("Crossover not present");

        let address =
            rusk_abi::contract_to_scalar(&ContractId::from_bytes(stct.module));

        let message =
            stct_signature_message(&crossover, stct.value, address).to_vec();
        let message = rusk_abi::poseidon_hash(message);

        let mut pi = Vec::with_capacity(6);

        pi.push(crossover.value_commitment().into());
        pi.push(stct.value.into());
        pi.push(stealth_addr.pk_r().as_ref().into());
        pi.push(message.into());

        //  1. v < 2^64
        //  2. B_a↦ = B_a↦ + v
        let contract_id = ContractId::from_bytes(stct.module);
        self.add_balance(contract_id, stct.value);

        //  3. if a.isPayable() ↦ true then continue
        let contract_id = ContractId::from_bytes(stct.module);
        match rusk_abi::payment_info(contract_id)
            .expect("Querying the payment info should succeed")
        {
            PaymentInfo::Transparent(_) | PaymentInfo::Any(_) => (),
            _ => panic!("The caller doesn't accept transparent notes"),
        }

        //  4. verify(C.c, v, π)
        let vd = verifier_data_stct();
        Self::assert_proof(vd, stct.proof, pi)
            .expect("Failed to verify the provided proof!");

        //  5. C ← C(0,0,0)
        //  Crossover is already taken

        true
    }

    pub fn withdraw_from_contract_transparent(
        &mut self,
        wfct: Wfct,
        from_address: ContractId,
    ) -> bool {
        let mut pi = Vec::with_capacity(3);

        pi.push(wfct.value.into());
        pi.push(wfct.note.value_commitment().into());

        //  1. a ∈ B↦
        //  2. B_a↦ ← B_a↦ − v
        self.sub_balance(&from_address, wfct.value)
            .expect("Failed to subtract the balance from the provided address");

        //  3. N↦.append(N_p^t)
        //  4. N_p^* ← encode(N_p^t)
        //  5. N.append(N_p^*)
        self.push_note_current_height(wfct.note);

        //  6. verify(C.c, M, pk, π)
        let vd = verifier_data_wfct();
        Self::assert_proof(vd, wfct.proof, pi)
            .expect("Failed to verify the provided proof!");

        true
    }

    pub fn withdraw_from_contract_transparent_raw(
        &mut self,
        wfct_raw: transfer_contract_types::WfctRaw,
        from_address: ContractId,
    ) -> bool {
        let note = Note::from_slice(wfct_raw.note.as_slice())
            .expect("Failed to deserialize note");
        self.withdraw_from_contract_transparent(
            Wfct {
                value: wfct_raw.value,
                note,
                proof: wfct_raw.proof,
            },
            from_address,
        )
    }

    pub fn send_to_contract_obfuscated(&mut self, stco: Stco) -> bool {
        let (crossover, stealth_addr) = self
            .take_crossover()
            .expect("The crossover is mandatory for STCO!");

        let contract_id = ContractId::from_bytes(stco.module);
        let module = rusk_abi::contract_to_scalar(&contract_id);

        let sign_message =
            stco_signature_message(&crossover, &stco.message, module).to_vec();
        let sign_message = rusk_abi::poseidon_hash(sign_message);

        let (message_psk_a, message_psk_b) =
            match rusk_abi::payment_info(contract_id)
                .expect("Querying the payment info should succeed")
            {
                PaymentInfo::Obfuscated(Some(k))
                | PaymentInfo::Any(Some(k)) => (*k.A(), *k.B()),

                PaymentInfo::Obfuscated(None) | PaymentInfo::Any(None) => {
                    (JubJubExtended::identity(), JubJubExtended::identity())
                }

                _ => panic!("The caller doesn't accept obfuscated notes"),
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
        pi.push(module.into());
        pi.push(sign_message.into());
        pi.push(stealth_addr.pk_r().as_ref().into());

        //  1. S_a↦.append((pk, R))
        //  2. M_a↦.M_pk↦.append(M)
        self.push_message(contract_id, stco.message_address, stco.message);

        //  3. if a.isPayable() → true, obf, psk_a? then continue
        //  4. verify(C.c, M, pk, π)
        let vd = verifier_data_stco();
        Self::assert_proof(vd, stco.proof, pi)
            .expect("Failed to verify the provided proof!");

        //  5. C←(0,0,0)
        //  Crossover is already taken

        true
    }

    pub fn withdraw_from_contract_obfuscated(
        &mut self,
        wfco: Wfco,
        from_address: ContractId,
    ) -> bool {
        let (change_psk_a, change_psk_b) =
            match rusk_abi::payment_info(from_address)
                .expect("Querying the payment info should succeed")
            {
                PaymentInfo::Obfuscated(Some(k))
                | PaymentInfo::Any(Some(k)) => (*k.A(), *k.B()),

                PaymentInfo::Obfuscated(None) | PaymentInfo::Any(None) => {
                    (JubJubExtended::identity(), JubJubExtended::identity())
                }

                _ => panic!("The caller doesn't accept obfuscated notes"),
            };

        let mut pi = alloc::vec![
            wfco.message.value_commitment().into(),
            wfco.change.value_commitment().into(),
            change_psk_a.into(),
            change_psk_b.into(),
            wfco.change_address.pk_r().as_ref().into(),
            wfco.change.nonce().into(),
        ];
        pi.extend(wfco.change.cipher().iter().map(|c| c.into()));
        pi.push(wfco.output.value_commitment().into());

        //  1. a ∈ M↦
        //  2. pk ∈ M_a↦
        //  3. M_a↦.delete(pk)
        self.take_message_from_address_key(
            &from_address,
            wfco.message_address.pk_r(),
        )
        .expect(
            "Failed to take a message from the provided address/key mapping!",
        );

        self.push_message(from_address, wfco.change_address, wfco.change);

        //  6. if a.isPayable() → true, obf, psk_a? then continue
        match rusk_abi::payment_info(from_address)
            .expect("Querying the payment info should succeed")
        {
            PaymentInfo::Obfuscated(_) | PaymentInfo::Any(_) => (),
            _ => panic!("This contract accepts only obfuscated notes!"),
        }

        self.push_note_current_height(wfco.output);

        //  7. verify(c, M_c, No.c, π)
        let vd = verifier_data_wfco();
        Self::assert_proof(vd, wfco.proof, pi)
            .expect("Failed to verify the provided proof!");

        true
    }

    pub fn withdraw_from_contract_obfuscated_raw(
        &mut self,
        wfco_raw: WfcoRaw,
        from_address: ContractId,
    ) -> bool {
        let output = Note::from_slice(wfco_raw.output.as_slice())
            .expect("Failed to deserialize note");
        self.withdraw_from_contract_obfuscated(
            Wfco {
                message: wfco_raw.message,
                message_address: wfco_raw.message_address,
                change: wfco_raw.change,
                change_address: wfco_raw.change_address,
                output,
                proof: wfco_raw.proof,
            },
            from_address,
        )
    }

    pub fn withdraw_from_contract_transparent_to_contract(
        &mut self,
        wfctc: Wfctc,
        from_address: ContractId,
    ) -> bool {
        //  1. from ∈ B↦
        //  2. B_from↦ ← B_from↦ − v
        self.sub_balance(&from_address, wfctc.value).expect(
            "Failed to subtract the balance from the provided address!",
        );

        //  3. B_to↦ = B_to↦ + v
        let module = ContractId::from_bytes(wfctc.module);
        self.add_balance(module, wfctc.value);

        true
    }

    /// Spends the inputs and creates the given UTXO.
    /// It performs all checks necessary to ensure the
    /// transaction is valid - hash matches, anchor has been a root of the
    /// tree, proof checks out, etc...
    ///
    /// This will emplace the crossover in the state, if it exists - making it
    /// available for any contracts called.
    ///
    /// [`refund`] **must** be called if this function succeeds, otherwise we
    /// will have an inconsistent state.
    ///
    /// # Panics
    /// Any failure in the checks performed in processing the transaction will
    /// result in a panic. The contract expects the environment to roll back any
    /// change in state.
    ///
    /// [`refund`]: [`TransferState::refund`]
    pub fn spend(&mut self, tx: Transaction) -> Result<Vec<u8>, ContractError> {
        //  1. α ∈ R
        if !self.root_exists(&tx.anchor) {
            panic!("Anchor not found in the state!");
        }

        //  2. ν[] !∈ Nullifiers
        let nullifier_exists = rusk_abi::call::<Vec<BlsScalar>, bool>(
            TRANSFER_DATA_CONTRACT,
            "any_nullifier_exists",
            &tx.nullifiers,
        )
        .expect("nullifiers query should succeed");
        if nullifier_exists {
            panic!("A provided nullifier already exists!");
        }

        //  3. Nullifiers.append(ν[])
        rusk_abi::call::<Vec<BlsScalar>, ()>(
            TRANSFER_DATA_CONTRACT,
            "extend_nullifiers",
            &tx.nullifiers,
        )
        .expect("extending nullifiers should succeed");

        //  4. if |C|=0 then set C ← (0,0,0)
        //  Crossover is received as option

        //  5. N↦.append((No.R[], No.pk[])
        //  6. Notes.append(No[])
        let block_height = rusk_abi::block_height();
        rusk_abi::call::<(u64, Vec<Note>), ()>(
            TRANSFER_DATA_CONTRACT,
            "extend_notes",
            &(block_height, tx.outputs.clone()),
        )
        .expect("extending notes should succeed");

        //  7. g_l < 2^64
        //  8. g_pmin < g_p
        //  9. fee ← g_l ⋅ g_p
        // 10. verify(α, ν[], C.c, No.c[], fee)
        if !verify_tx_proof(&tx) {
            panic!("Invalid transaction proof!");
        }

        // 11. if ∣k∣≠0 then call(k)
        rusk_abi::call::<(Option<Crossover>, Option<StealthAddress>), ()>(
            TRANSFER_DATA_CONTRACT,
            "set_crossover",
            &(tx.crossover, Some(*tx.fee.stealth_address())),
        )
        .expect("set_crossover call should succeed");

        Ok(Vec::new())
    }
    /// Executes the contract call if present.
    ///
    /// This function guarantees that it will not panic.
    pub fn execute(
        &mut self,
        tx: Transaction,
    ) -> Result<Vec<u8>, ContractError> {
        let mut result = Ok(Vec::new());

        if let Some((contract_id, fn_name, fn_args)) = tx.call {
            if contract_id == TRANSFER_DATA_CONTRACT.to_bytes() {
                return Err(ContractError::Panic("Transfer data contract can only be called from the transfer contract".to_string()));
            }
            if contract_id == TRANSFER_LOGIC_CONTRACT.to_bytes() {
                return Err(ContractError::Panic("Transfer logic contract can only be called from the transfer contract".to_string()));
            }
            result = rusk_abi::call_raw(
                ContractId::from_bytes(contract_id),
                &fn_name,
                &fn_args,
            );
        }

        result
    }

    /// Refund the previously performed transaction, taking into account the
    /// given gas spent. The notes produced will be refunded to the address
    /// present in the fee structure.
    ///
    /// This function guarantees that it will not panic.
    pub fn refund(&mut self, fee: Fee, gas_spent: u64) {
        let block_height = rusk_abi::block_height();

        let remainder = fee.gen_remainder(gas_spent);
        let remainder = Note::from(remainder);

        let remainder_value = remainder
            .value(None)
            .expect("Should always succeed for a transparent note");

        if remainder_value > 0 {
            self.push_note(block_height, remainder);
        }

        let (crossover, _) = rusk_abi::call::<
            (),
            (Option<Crossover>, Option<StealthAddress>),
        >(
            TRANSFER_DATA_CONTRACT, "get_crossover", &()
        )
        .expect("get_crossover call should succeed");
        if let Some(crossover) = crossover {
            let note = Note::from((fee, crossover));
            self.push_note(block_height, note);
        }
    }

    /// Push a note to the contract's state with the given block height
    ///
    /// Note: the method `update_root` needs to be called after the last note is
    /// pushed.
    pub fn push_note(&mut self, block_height: u64, note: Note) -> Note {
        let pos = rusk_abi::call::<(u64, Note), u64>(
            TRANSFER_DATA_CONTRACT,
            "push_note",
            &(block_height, note),
        )
        .expect("push_note call should succeed");
        let tree_leaf = TreeLeaf { block_height, note };
        rusk_abi::emit("TREE_LEAF", (pos, tree_leaf));
        self.get_note(pos)
            .expect("There should be a note that was just inserted")
    }

    /// Feeds the host with the leaves in the tree, starting from the given
    /// height.
    pub fn leaves_from_height(&self, height: u64) {
        rusk_abi::call::<u64, ()>(
            TRANSFER_DATA_CONTRACT,
            "leaves_from_height",
            &height,
        )
        .expect("leaves_from_height query should succeed");
    }

    /// Feeds the host with the leaves in the tree, starting from the given
    /// position.
    pub fn leaves_from_pos(&self, pos: u64) {
        rusk_abi::call::<u64, ()>(
            TRANSFER_DATA_CONTRACT,
            "leaves_from_pos",
            &pos,
        )
        .expect("leaves_from_pos query should succeed");
    }

    /// Update the root of the tree.
    pub fn update_root(&mut self) {
        rusk_abi::call::<(), ()>(TRANSFER_DATA_CONTRACT, "update_root", &())
            .expect("update_root call should succeed");
    }

    /// Get the root of the tree.
    pub fn root(&self) -> BlsScalar {
        rusk_abi::call::<(), BlsScalar>(TRANSFER_DATA_CONTRACT, "root", &())
            .expect("root query should succeed")
    }

    /// Get the count of the notes in the tree.
    pub fn num_notes(&self) -> u64 {
        rusk_abi::call::<(), u64>(TRANSFER_DATA_CONTRACT, "num_notes", &())
            .expect("num_notes query should succeed")
    }

    /// Get the opening
    pub fn opening(
        &self,
        pos: u64,
    ) -> Option<PoseidonOpening<(), TRANSFER_TREE_DEPTH, A>> {
        rusk_abi::call::<u64, Option<PoseidonOpening<(), TRANSFER_TREE_DEPTH, A>>>(
            TRANSFER_DATA_CONTRACT,
            "opening",
            &pos,
        )
        .expect("opening query should succeed")
    }

    /// Takes some nullifiers and returns a vector containing the ones that
    /// already exists in the contract
    pub fn existing_nullifiers(
        &self,
        nullifiers: &Vec<BlsScalar>,
    ) -> Vec<BlsScalar> {
        rusk_abi::call::<Vec<BlsScalar>, Vec<BlsScalar>>(
            TRANSFER_DATA_CONTRACT,
            "existing_nullifiers",
            nullifiers,
        )
        .expect("calling existing nullifiers should succeed")
    }

    /// Return the balance of a given contract.
    pub fn balance(&self, contract_id: &ContractId) -> u64 {
        rusk_abi::call(
            TRANSFER_DATA_CONTRACT,
            "get_module_balance",
            contract_id,
        )
        .expect("balance query should succeed")
    }

    /// Add balance to the given contract
    pub fn add_balance(&mut self, contract: ContractId, value: u64) {
        rusk_abi::call::<(ContractId, u64), ()>(
            TRANSFER_DATA_CONTRACT,
            "add_module_balance",
            &(contract, value),
        )
        .expect("add_module_balance call should succeed");
    }

    pub fn message(
        &self,
        contract: &ContractId,
        pk: &PublicKey,
    ) -> Option<Message> {
        rusk_abi::call::<(ContractId, PublicKey), Option<Message>>(
            TRANSFER_DATA_CONTRACT,
            "message",
            &(*contract, *pk),
        )
        .expect("message call should succeed")
    }

    fn get_note(&self, pos: u64) -> Option<Note> {
        rusk_abi::call::<u64, Option<Note>>(
            TRANSFER_DATA_CONTRACT,
            "get_note",
            &pos,
        )
        .expect("get_note query should succeed")
    }

    fn take_message_from_address_key(
        &mut self,
        contract: &ContractId,
        pk: &PublicKey,
    ) -> Result<Message, Error> {
        rusk_abi::call::<(ContractId, PublicKey), Option<Message>>(
            TRANSFER_DATA_CONTRACT,
            "take_message_from_address_key",
            &(*contract, *pk),
        )
        .expect("take_message_from_address_key call should succeed")
        .ok_or(Error::MessageNotFound)
    }

    fn root_exists(&self, root: &BlsScalar) -> bool {
        rusk_abi::call::<BlsScalar, bool>(
            TRANSFER_DATA_CONTRACT,
            "root_exists",
            root,
        )
        .expect("root_exists query should succeed")
    }

    fn push_note_current_height(&mut self, note: Note) -> Note {
        let block_height = rusk_abi::block_height();
        self.push_note(block_height, note)
    }

    pub(crate) fn sub_balance(
        &mut self,
        address: &ContractId,
        value: u64,
    ) -> Result<(), Error> {
        rusk_abi::call::<(ContractId, u64), Option<()>>(
            TRANSFER_DATA_CONTRACT,
            "sub_balance",
            &(*address, value),
        )
        .expect("sub_balance call should succeed")
        .ok_or(Error::NotEnoughBalance)
    }

    fn push_message(
        &mut self,
        address: ContractId,
        message_address: StealthAddress,
        message: Message,
    ) {
        rusk_abi::call::<(ContractId, StealthAddress, Message), ()>(
            TRANSFER_DATA_CONTRACT,
            "push_message",
            &(address, message_address, message),
        )
        .expect("push_message call should succeed");
    }

    fn take_crossover(&mut self) -> Result<(Crossover, StealthAddress), Error> {
        rusk_abi::call::<(), Option<(Crossover, StealthAddress)>>(
            TRANSFER_DATA_CONTRACT,
            "take_crossover",
            &(),
        )
        .expect("take_crossover call should succeed")
        .ok_or(Error::CrossoverNotFound)
    }

    fn assert_proof(
        verifier_data: &[u8],
        proof: Vec<u8>,
        public_inputs: Vec<PublicInput>,
    ) -> Result<(), Error> {
        rusk_abi::verify_proof(verifier_data.to_vec(), proof, public_inputs)
            .then_some(())
            .ok_or(Error::ProofVerification)
    }
}

fn verify_tx_proof(tx: &Transaction) -> bool {
    // Constant for a pedersen commitment with zero value.
    // Calculated as `G^0 · G'^0`
    const ZERO_COMMITMENT: JubJubAffine =
        JubJubAffine::from_raw_unchecked(BlsScalar::zero(), BlsScalar::one());

    let n_nullifiers = tx.nullifiers.len();
    let n_outputs = tx.outputs.len();

    let tx_hash = rusk_abi::hash(tx.to_hash_input_bytes());
    let crossover_commitment = tx
        .crossover
        .map(|c| *c.value_commitment())
        .unwrap_or_default();
    let fee_value = tx.fee.gas_limit * tx.fee.gas_price;

    let mut pis =
        Vec::<PublicInput>::with_capacity(5 + n_nullifiers + 2 * n_outputs);

    pis.push(tx_hash.into());
    pis.push(tx.anchor.into());
    pis.extend(tx.nullifiers.iter().map(Into::into));
    pis.push(crossover_commitment.into());

    pis.push(fee_value.into());
    pis.extend(tx.outputs.iter().map(|n| n.value_commitment().into()));
    pis.extend(
        (0usize..2usize.saturating_sub(n_outputs))
            .map(|_| ZERO_COMMITMENT.into()),
    );

    let vd = verifier_data_execute(n_nullifiers)
        .expect("No circuit available for given number of inputs!")
        .to_vec();
    rusk_abi::verify_proof(vd, tx.proof.clone(), pis)
}
