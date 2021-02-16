// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{PublicKeyBytes, Transfer};
use core::convert::TryFrom;

use alloc::vec::Vec;
use canonical::{InvalidEncoding, Store};
use dusk_bls12_381::BlsScalar;
use dusk_bytes::Serializable;
use dusk_jubjub::{JubJubAffine, JubJubScalar};
use dusk_kelvin_map::Map;
use dusk_pki::PublicKey;
use phoenix_core::{Crossover, Fee, Message, Note};

// FIXME provisory solution until this issue is fixed
// https://github.com/dusk-network/rusk-vm/issues/123
pub fn extend_pi_bls_scalar(pi: &mut Vec<u8>, s: &BlsScalar) {
    pi.push(0x01);
    pi.extend_from_slice(&s.to_bytes());
}

// FIXME provisory solution until this issue is fixed
// https://github.com/dusk-network/rusk-vm/issues/123
pub fn extend_pi_jubjub_scalar(pi: &mut Vec<u8>, s: &JubJubScalar) {
    pi.push(0x02);
    pi.extend_from_slice(&s.to_bytes());
}

// FIXME provisory solution until this issue is fixed
// https://github.com/dusk-network/rusk-vm/issues/123
pub fn extend_pi_jubjub_affine(pi: &mut Vec<u8>, p: &JubJubAffine) {
    pi.push(0x03);
    pi.extend_from_slice(&p.to_bytes());
}

impl<S: Store> Transfer<S> {
    // TODO should be const fn after rust stabilize the API
    // https://github.com/rust-lang/rust/issues/57563
    pub(crate) fn rusk_label(inputs: usize, outputs: usize) -> &'static str {
        match (inputs, outputs) {
            (1, 0) => "transfer-execute-1-0",
            (1, 1) => "transfer-execute-1-1",
            (1, 2) => "transfer-execute-1-2",
            (2, 0) => "transfer-execute-2-0",
            (2, 1) => "transfer-execute-2-1",
            (2, 2) => "transfer-execute-2-2",
            (3, 0) => "transfer-execute-3-0",
            (3, 1) => "transfer-execute-3-1",
            (3, 2) => "transfer-execute-3-2",
            (4, 0) => "transfer-execute-4-0",
            (4, 1) => "transfer-execute-4-1",
            (4, 2) => "transfer-execute-4-2",
            _ => "unimplemented",
        }
    }

    pub(crate) fn push_fee_crossover(
        &mut self,
        fee: Fee,
        crossover: Option<Crossover>,
    ) -> Result<(), S::Error> {
        // TODO Get gas consumed
        let gas_consumed = 1;
        let remainder = fee.gen_remainder(gas_consumed);

        self.push_note(remainder.into())?;

        if let Some(crossover) = crossover {
            Note::try_from((fee, crossover))
                .map_err(|_| InvalidEncoding.into())
                .and_then(|note| self.push_note(note))?;
        }

        Ok(())
    }

    // TODO convert to const fn
    // https://github.com/rust-lang/rust/issues/57563
    pub(crate) fn minimum_gas_price() -> u64 {
        // TODO define the mininum gas price
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

    pub(crate) fn push_note(&mut self, note: Note) -> Result<(), S::Error> {
        let block_height = dusk_abi::block_height();

        let mut create = false;
        match self.notes_mapping.get_mut(&block_height)? {
            // TODO evaluate options for efficient dedup
            // We can't call dedup here because the note `PartialEq` relies on
            // poseidon hash, that is supposed to be a host function
            Some(mut mapped) => mapped.push(note.clone()),

            None => create = true,
        }
        if create {
            self.notes_mapping.insert(block_height, [note].to_vec())?;
        }

        self.notes
            .push((block_height, note).into())
            .map(|_| ())
            .map_err(|_| InvalidEncoding.into())
    }

    pub(crate) fn extend_notes(
        &mut self,
        notes: Vec<Note>,
    ) -> Result<(), S::Error> {
        let block_height = dusk_abi::block_height();

        let mut create = false;
        match self.notes_mapping.get_mut(&block_height)? {
            // TODO evaluate options for efficient dedup
            // We can't call dedup here because the note `PartialEq` relies on
            // poseidon hash, that is supposed to be a host function
            Some(mut mapped) => mapped.extend_from_slice(notes.as_slice()),

            None => create = true,
        }
        if create {
            self.notes_mapping.insert(block_height, notes.clone())?;
        }

        for note in notes {
            self.notes
                .push((block_height, note).into())
                .map_err(|_| InvalidEncoding.into())?;
        }

        Ok(())
    }

    pub(crate) fn add_balance(
        &mut self,
        address: BlsScalar,
        value: u64,
    ) -> Result<(), S::Error> {
        if let Some(mut balance) = self.balance.get_mut(&address)? {
            *balance += value;
            return Ok(());
        }

        self.balance.insert(address, value)?;
        Ok(())
    }

    pub(crate) fn sub_balance(
        &mut self,
        address: BlsScalar,
        value: u64,
    ) -> Result<(), S::Error> {
        match self.balance.get_mut(&address)? {
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
}
