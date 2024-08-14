// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Implementations of basic wallet functionalities.

#![cfg_attr(target_family = "wasm", no_std)]
#![deny(missing_docs)]
#![deny(rustdoc::broken_intra_doc_links)]
#![deny(clippy::pedantic)]

extern crate alloc;

pub mod keys;

/// Length of the seed of the generated rng.
pub const RNG_SEED: usize = 64;

// The maximum amount of input notes that can be spend in one
// phoenix-transaction
const MAX_INPUT_NOTES: usize = 4;

use alloc::vec::Vec;

use execution_core::transfer::phoenix::{Note, ViewKey as PhoenixViewKey};

/// Calculate the sum for all the given [`Note`]s that belong to the given
/// [`PhoenixViewKey`].
pub fn phoenix_balance(
    phoenix_vk: &PhoenixViewKey,
    notes: impl AsRef<[Note]>,
) -> BalanceInfo {
    let mut values: Vec<u64> = Vec::new();
    notes.as_ref().iter().for_each(|note| {
        values.push(note.value(Some(phoenix_vk)).unwrap_or_default());
    });

    values.sort_by(|a, b| b.cmp(a));

    BalanceInfo {
        value: values.iter().sum(),
        spendable: values[..MAX_INPUT_NOTES].iter().sum(),
    }
}

/// Information about the balance of a particular key.
#[derive(Debug, Default, Hash, Clone, Copy, PartialEq, Eq)]
pub struct BalanceInfo {
    /// The total value of the balance.
    pub value: u64,
    /// The maximum _spendable_ value in a single transaction. This is
    /// different from `value` since there is a maximum number of notes one can
    /// spend.
    pub spendable: u64,
}
