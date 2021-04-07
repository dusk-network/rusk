// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{PublicKeyBytes, TransferContract};
use core::convert::TryFrom;

use alloc::vec::Vec;
use canonical::{InvalidEncoding, Store};
use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;
use dusk_jubjub::JubJubAffine;
use dusk_kelvin_map::Map;
use dusk_pki::PublicKey;
use phoenix_core::{Crossover, Fee, Message, Note};
use rusk_abi::PublicInput;

impl<S: Store> TransferContract<S> {
    pub(crate) fn push_fee_crossover(
        &mut self,
        fee: Fee,
    ) -> Result<(), S::Error> {
        let block_height = dusk_abi::block_height();

        // FIXME Get gas consumed
        // https://github.com/dusk-network/rusk/issues/195
        let gas_consumed = 2;
        let remainder = fee.gen_remainder(gas_consumed);
        let remainder = Note::from(remainder);
        let remainder_value =
            remainder.value(None).map_err(|_| InvalidEncoding.into())?;
        if remainder_value > 0 {
            self.push_note(block_height, remainder)?;
        }

        if let Some(crossover) = self.var_crossover {
            Note::try_from((fee, crossover))
                .map_err(|_| InvalidEncoding.into())
                .and_then(|note| self.push_note(block_height, note))?;
        }

        Ok(())
    }

    // TODO convert to const fn
    // https://github.com/rust-lang/rust/issues/57563
    pub(crate) fn minimum_gas_price() -> u64 {
        // FIXME define the mininum gas price
        // https://github.com/dusk-network/rusk/issues/195
        0
    }

    pub(crate) fn any_nullifier_exists(
        &self,
        nullifiers: &[BlsScalar],
    ) -> Result<bool, S::Error> {
        nullifiers.iter().try_fold(false, |t, n| {
            Ok(t || self.nullifiers.get(n).map(|n| n.is_some())?)
        })
    }

    pub(crate) fn root_exists(
        &self,
        root: &BlsScalar,
    ) -> Result<bool, S::Error> {
        self.roots.get(root).map(|t| t.is_some())
    }

    pub(crate) fn extend_nullifiers(
        &mut self,
        nullifiers: Vec<BlsScalar>,
    ) -> Result<(), S::Error> {
        for nullifier in nullifiers {
            self.nullifiers.insert(nullifier, ())?;
        }

        Ok(())
    }

    pub(crate) fn take_message_from_address_key(
        &mut self,
        address: &BlsScalar,
        pk: &PublicKey,
    ) -> Result<Message, S::Error> {
        self.message_mapping
            .get_mut(address)?
            .ok_or(InvalidEncoding.into())?
            .remove(&(*pk).to_bytes())?
            .ok_or(InvalidEncoding.into())
    }

    pub(crate) fn push_note_current_height(
        &mut self,
        note: Note,
    ) -> Result<Note, S::Error> {
        let block_height = dusk_abi::block_height();

        self.push_note(block_height, note)
    }

    pub(crate) fn extend_notes(
        &mut self,
        notes: Vec<Note>,
    ) -> Result<(), S::Error> {
        let block_height = dusk_abi::block_height();

        for note in notes {
            self.push_note(block_height, note)?;
        }

        Ok(())
    }

    pub(crate) fn add_balance(
        &mut self,
        address: BlsScalar,
        value: u64,
    ) -> Result<(), S::Error> {
        if let Some(mut balance) = self.balances.get_mut(&address)? {
            *balance += value;

            return Ok(());
        }

        self.balances.insert(address, value)?;

        Ok(())
    }

    pub(crate) fn sub_balance(
        &mut self,
        address: BlsScalar,
        value: u64,
    ) -> Result<(), S::Error> {
        match self.balances.get_mut(&address)? {
            Some(mut balance) if value <= *balance => {
                *balance -= value;

                Ok(())
            }

            _ => Err(InvalidEncoding.into()),
        }
    }

    pub(crate) fn push_message(
        &mut self,
        address: BlsScalar,
        pk: PublicKey,
        r: JubJubAffine,
        message: Message,
    ) -> Result<(), S::Error> {
        let mut to_insert: Option<Map<PublicKeyBytes, Message, S>> = None;

        match self.message_mapping.get_mut(&address)? {
            Some(mut map) => {
                map.insert(pk.to_bytes(), message)?;
            }

            None => {
                let mut map: Map<PublicKeyBytes, Message, S> = Map::default();
                map.insert(pk.to_bytes(), message)?;
                to_insert.replace(map);
            }
        }

        if let Some(map) = to_insert {
            self.message_mapping.insert(address, map)?;
        }

        self.message_mapping_set.insert(address, (pk, r))?;

        Ok(())
    }

    pub(crate) fn take_crossover(
        &mut self,
    ) -> Result<(Crossover, PublicKey), S::Error> {
        let crossover =
            self.var_crossover.take().ok_or(InvalidEncoding.into())?;
        let pk = self.var_crossover_pk.take().ok_or(InvalidEncoding.into())?;

        Ok((crossover, pk))
    }

    pub(crate) fn assert_payable(_address: &BlsScalar) -> Result<(), S::Error> {
        //  FIXME Use isPayable definition
        //  https://github.com/dusk-network/rusk-vm/issues/151

        Ok(())
    }

    pub(crate) fn assert_proof(
        proof: Vec<u8>,
        vd: &[u8],
        pi: Vec<PublicInput>,
    ) -> Result<(), S::Error> {
        rusk_abi::verify_proof(proof, vd.to_vec(), pi)
            .then(|| ())
            .ok_or(InvalidEncoding.into())
    }
}
