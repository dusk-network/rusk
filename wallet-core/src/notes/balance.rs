// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Provides functions and types for calculate notes' balance.

use alloc::vec::Vec;

use dusk_bytes::{DeserializableSlice, Serializable, Write};
use dusk_core::transfer::phoenix::{Note, ViewKey as PhoenixViewKey};

use crate::notes::MAX_INPUT_NOTES;

/// Calculate the sum for all the given [`Note`]s that belong to the given
/// [`PhoenixViewKey`].
pub fn calculate<T>(
    vk: &PhoenixViewKey,
    notes: impl Iterator<Item = T>,
) -> TotalAmount
where
    T: AsRef<Note>,
{
    let mut values: Vec<u64> = notes
        .filter_map(|note| {
            vk.owns(note.as_ref().stealth_address())
                .then_some(true)
                .and(note.as_ref().value(Some(vk)).ok())
        })
        .collect();

    values.sort_by(|a, b| b.cmp(a));

    let spendable = values.iter().take(MAX_INPUT_NOTES).sum();
    let value = spendable + values.iter().skip(MAX_INPUT_NOTES).sum::<u64>();

    TotalAmount { value, spendable }
}

/// Calculate the sum for all the given [`Note`]s without
/// performing any ownership checks. The [`PhoenixViewKey`]
/// is used solely for decrypting the values of obfuscated
/// notes.
pub fn calculate_unchecked<T>(
    vk: &PhoenixViewKey,
    notes: impl Iterator<Item = T>,
) -> TotalAmount
where
    T: AsRef<Note>,
{
    let mut values: Vec<u64> = notes
        .filter_map(|note| note.as_ref().value(Some(vk)).ok())
        .collect();

    values.sort_by(|a, b| b.cmp(a));

    let spendable = values.iter().take(MAX_INPUT_NOTES).sum();
    let value = spendable + values.iter().skip(MAX_INPUT_NOTES).sum::<u64>();

    TotalAmount { value, spendable }
}

/// Information about the balance of a particular key.
#[derive(Debug, Default, Hash, Clone, Copy, PartialEq, Eq)]
pub struct TotalAmount {
    /// The total value of the balance.
    pub value: u64,
    /// The maximum _spendable_ value in a single transaction. This is
    /// different from `value` since there is a maximum number of notes one can
    /// spend.
    pub spendable: u64,
}

impl Serializable<{ 2 * u64::SIZE }> for TotalAmount {
    type Error = dusk_bytes::Error;

    fn from_bytes(buf: &[u8; Self::SIZE]) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        let mut reader = &buf[..];

        let value = u64::from_reader(&mut reader)?;
        let spendable = u64::from_reader(&mut reader)?;

        Ok(Self { value, spendable })
    }

    #[allow(unused_must_use)]
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];
        let mut writer = &mut buf[..];

        writer.write(&self.value.to_bytes());
        writer.write(&self.spendable.to_bytes());

        buf
    }
}
