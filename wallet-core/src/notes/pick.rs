// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Helper functions for working with notes.

use alloc::vec::Vec;

use crate::notes::owned::NoteList;
use crate::notes::MAX_INPUT_NOTES;
use execution_core::transfer::phoenix::{NoteLeaf, ViewKey as PhoenixViewKey};
use execution_core::BlsScalar;

/// Pick the notes to be used in a transaction from a vector of notes.
///
/// The notes are picked in a way to maximize the number of notes used,
/// while minimizing the value employed. To do this we sort the notes in
/// ascending value order, and go through each combination in a
/// lexicographic order until we find the first combination whose sum is
/// larger or equal to the given value. If such a slice is not found, an
/// empty vector is returned.
///
/// If the target sum is greater than the sum of the notes then an
/// empty vector is returned.
#[must_use]
pub fn notes(vk: &PhoenixViewKey, notes: NoteList, value: u64) -> NoteList {
    if notes.is_empty() {
        return NoteList::default();
    }

    let mut notes_and_values: Vec<(NoteLeaf, u64, BlsScalar)> = notes
        .iter()
        .filter_map(|(nullifier, leaf)| {
            leaf.as_ref()
                .value(Some(vk))
                .ok()
                .map(|value| (leaf.clone(), value, *nullifier))
        })
        .collect();

    let sum: u64 = notes_and_values
        .iter()
        .fold(0, |sum, &(_, value, _)| sum.saturating_add(value));

    if sum < value {
        return NoteList::default();
    }

    if notes.len() <= MAX_INPUT_NOTES {
        return notes;
    }

    notes_and_values.sort_by(|(_, aval, _), (_, bval, _)| aval.cmp(bval));
    pick_lexicographic(notes_and_values.len(), |indices| {
        indices
            .iter()
            .map(|index| notes_and_values[*index].1)
            .sum::<u64>()
            >= value
    })
    .map(|index| notes_and_values[index].clone())
    .map(|(n, _, b)| (b, n))
    .to_vec()
    .into()
}

fn pick_lexicographic<F: Fn(&[usize; MAX_INPUT_NOTES]) -> bool>(
    max_len: usize,
    is_valid: F,
) -> [usize; MAX_INPUT_NOTES] {
    let mut indices = [0; MAX_INPUT_NOTES];
    indices
        .iter_mut()
        .enumerate()
        .for_each(|(i, index)| *index = i);

    loop {
        if is_valid(&indices) {
            return indices;
        }

        let mut i = MAX_INPUT_NOTES - 1;

        while indices[i] == i + max_len - MAX_INPUT_NOTES {
            if i > 0 {
                i -= 1;
            } else {
                break;
            }
        }

        indices[i] += 1;
        for j in i + 1..MAX_INPUT_NOTES {
            indices[j] = indices[j - 1] + 1;
        }

        if indices[MAX_INPUT_NOTES - 1] == max_len {
            break;
        }
    }

    [0; MAX_INPUT_NOTES]
}
