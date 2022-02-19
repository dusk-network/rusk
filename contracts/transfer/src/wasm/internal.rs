// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{Error, Map, PublicKeyBytes, TransferContract};

use alloc::vec::Vec;
use dusk_abi::ContractId;
use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;
use dusk_pki::{PublicKey, StealthAddress};
use phoenix_core::{Crossover, Fee, Message, Note};
use rusk_abi::dusk::*;
use rusk_abi::PublicInput;

impl TransferContract {
    pub(crate) fn push_fee_crossover(&mut self, fee: Fee) -> Result<(), Error> {
        let block_height = dusk_abi::block_height();

        let gas_left = dusk_abi::gas_left();
        let remainder = fee.gen_remainder(fee.gas_limit - gas_left);
        let remainder = Note::from(remainder);
        let remainder_value = remainder.value(None)?;
        if remainder_value > 0 {
            self.push_note(block_height, remainder)?;
        }

        if let Some(crossover) = self.var_crossover {
            let note = Note::from((fee, crossover));
            self.push_note(block_height, note)?;
        }

        Ok(())
    }

    /// Minimum accepted price per unit of gas.
    pub(crate) const fn minimum_gas_price() -> Dusk {
        LUX
    }

    pub(crate) fn root_exists(&self, root: &BlsScalar) -> Result<bool, Error> {
        let root = self.roots.get(root)?;

        Ok(root.is_some())
    }

    pub(crate) fn extend_nullifiers(
        &mut self,
        nullifiers: Vec<BlsScalar>,
    ) -> Result<(), Error> {
        for nullifier in nullifiers {
            self.nullifiers.insert(nullifier, ())?;
        }

        Ok(())
    }

    pub(crate) fn take_message_from_address_key(
        &mut self,
        address: &ContractId,
        pk: &PublicKey,
    ) -> Result<Message, Error> {
        self.message_mapping
            .get_mut(address)?
            .ok_or(Error::MessageNotFound)?
            .remove(&pk.to_bytes())?
            .ok_or(Error::MessageNotFound)
    }

    pub(crate) fn push_note_current_height(
        &mut self,
        note: Note,
    ) -> Result<Note, Error> {
        let block_height = dusk_abi::block_height();

        self.push_note(block_height, note)
    }

    pub(crate) fn extend_notes(
        &mut self,
        notes: Vec<Note>,
    ) -> Result<(), Error> {
        let block_height = dusk_abi::block_height();

        for note in notes {
            self.push_note(block_height, note)?;
        }

        Ok(())
    }

    pub(crate) fn add_balance(
        &mut self,
        address: ContractId,
        value: u64,
    ) -> Result<(), Error> {
        if let Some(mut balance) = self.balances.get_mut(&address)? {
            *balance += value;

            return Ok(());
        }

        self.balances.insert(address, value)?;

        Ok(())
    }

    pub(crate) fn sub_balance(
        &mut self,
        address: &ContractId,
        value: u64,
    ) -> Result<(), Error> {
        // TODO workaround until deref is implemented for microkelvin branch
        // mapped mut
        use core::ops::DerefMut;

        match self.balances.get_mut(address)? {
            Some(mut balance) => {
                let bal_ref = balance.deref_mut();
                let (bal, underflow) = bal_ref.overflowing_sub(value);

                if underflow {
                    Err(Error::NotEnoughBalance)
                } else {
                    *bal_ref = bal;

                    Ok(())
                }
            }

            _ => Err(Error::NotEnoughBalance),
        }
    }

    pub(crate) fn push_message(
        &mut self,
        address: ContractId,
        message_address: StealthAddress,
        message: Message,
    ) -> Result<(), Error> {
        let mut to_insert: Option<Map<PublicKeyBytes, Message>> = None;

        match self.message_mapping.get_mut(&address)? {
            Some(mut map) => {
                map.insert(message_address.pk_r().to_bytes(), message)?;
            }

            None => {
                let mut map: Map<PublicKeyBytes, Message> = Map::default();
                map.insert(message_address.pk_r().to_bytes(), message)?;
                to_insert.replace(map);
            }
        }

        if let Some(map) = to_insert {
            self.message_mapping.insert(address, map)?;
        }

        self.message_mapping_set.insert(address, message_address)?;

        Ok(())
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
        proof: Vec<u8>,
        vd: &[u8],
        pi: Vec<PublicInput>,
    ) -> Result<(), Error> {
        rusk_abi::verify_proof(proof, vd.to_vec(), pi)
            .then(|| ())
            .ok_or(Error::ProofVerificationError)
    }
}
