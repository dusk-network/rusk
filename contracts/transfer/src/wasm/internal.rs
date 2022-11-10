// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::error::Error;
use crate::transfer::TransferState;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;
use dusk_pki::{PublicKey, StealthAddress};
use dusk_plonk::prelude::Proof;
use phoenix_core::{Crossover, Fee, Message, Note};

use rusk_abi::{dusk::*, ModuleId, PublicInput};

impl TransferState {
    pub(crate) fn push_fee_crossover(&mut self, fee: Fee) -> Result<(), Error> {
        let block_height = rusk_abi::block_height();

        let gas_left = rusk_abi::limit() - rusk_abi::spent();
        let remainder = fee.gen_remainder(fee.gas_limit - gas_left);
        let remainder = Note::from(remainder);
        let remainder_value = remainder.value(None)?;
        if remainder_value > 0 {
            self.push_note(block_height, remainder);
        }

        if let Some(crossover) = self.var_crossover {
            let note = Note::from((fee, crossover));
            self.push_note(block_height, note);
        }

        Ok(())
    }

    /// Minimum accepted price per unit of gas.
    pub(crate) const fn minimum_gas_price() -> Dusk {
        LUX
    }

    pub(crate) fn root_exists(&self, root: &BlsScalar) -> bool {
        self.roots.get(root).is_some()
    }

    pub(crate) fn extend_nullifiers(&mut self, nullifiers: Vec<BlsScalar>) {
        self.nullifiers.extend(nullifiers);
    }

    pub(crate) fn take_message_from_address_key(
        &mut self,
        address: &ModuleId,
        pk: &PublicKey,
    ) -> Result<Message, Error> {
        self.message_mapping
            .get_mut(address)
            .ok_or(Error::MessageNotFound)?
            .remove(&pk.to_bytes())
            .ok_or(Error::MessageNotFound)
    }

    pub(crate) fn push_note_current_height(&mut self, note: Note) -> Note {
        let block_height = rusk_abi::block_height();
        self.push_note(block_height, note)
    }

    pub(crate) fn extend_notes(&mut self, notes: Vec<Note>) {
        let block_height = rusk_abi::block_height();

        for note in notes {
            self.push_note(block_height, note);
        }
    }

    pub(crate) fn sub_balance(
        &mut self,
        address: &ModuleId,
        value: u64,
    ) -> Result<(), Error> {
        match self.balances.get_mut(address) {
            Some(balance) => {
                let (bal, underflow) = balance.overflowing_sub(value);

                if underflow {
                    Err(Error::NotEnoughBalance)
                } else {
                    *balance = bal;

                    Ok(())
                }
            }

            _ => Err(Error::NotEnoughBalance),
        }
    }

    pub(crate) fn push_message(
        &mut self,
        address: ModuleId,
        message_address: StealthAddress,
        message: Message,
    ) {
        let mut to_insert: Option<BTreeMap<[u8; PublicKey::SIZE], Message>> =
            None;

        match self.message_mapping.get_mut(&address) {
            Some(map) => {
                map.insert(message_address.pk_r().to_bytes(), message);
            }

            None => {
                let mut map: BTreeMap<[u8; PublicKey::SIZE], Message> =
                    BTreeMap::default();
                map.insert(message_address.pk_r().to_bytes(), message);
                to_insert.replace(map);
            }
        }

        if let Some(map) = to_insert {
            self.message_mapping.insert(address, map);
        }

        self.message_mapping_set.insert(address, message_address);
    }

    pub(crate) fn take_crossover(
        &mut self,
    ) -> Result<(Crossover, PublicKey), Error> {
        let crossover =
            self.var_crossover.take().ok_or(Error::CrossoverNotFound)?;

        let pk = self
            .var_crossover_pk
            .take()
            .ok_or(Error::CrossoverNotFound)?;

        Ok((crossover, pk))
    }

    pub(crate) fn assert_proof(
        verifier_data: &[u8],
        proof: Proof,
        public_inputs: Vec<PublicInput>,
    ) -> Result<(), Error> {
        rusk_abi::verify_proof(verifier_data.to_vec(), proof, public_inputs)
            .then(|| ())
            .ok_or(Error::ProofVerificationError)
    }
}
