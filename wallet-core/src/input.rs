// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Helper functions for working with notes.

use alloc::vec::Vec;

use super::{alloc, Note};

use execution_core::BlsScalar;

/// The maximum amount of input notes that can be spend in one
/// phoenix-transaction
pub const MAX_INPUT_NOTES: usize = 4;

/// Pick the notes to be used in a transaction from a vector of notes.
///
/// The resulting array is only 4 notes long, the argument of this function can
/// be arbitary amount of notes.
///
/// # Errors
///
/// If the target sum is greater than the sum of the notes then an error is
/// returned. If the notes vector is empty then an error is returned.
///
/// See `InputNotesError` type for possible errors
/// this function can yield.
#[must_use]
pub fn try_input_notes(
    nodes: Vec<(Note, u64, BlsScalar)>,
    target_sum: u64,
) -> Vec<(Note, BlsScalar)> {
    if nodes.is_empty() {
        return Vec::new();
    }

    let mut i = 0;
    let mut sum = 0;
    while sum < target_sum && i < nodes.len() {
        sum = sum.saturating_add(nodes[i].1);
        i += 1;
    }

    if sum < target_sum {
        return Vec::new();
    }

    pick_notes(target_sum, nodes)
}

/// Pick the notes to be used in a transaction from a vector of notes.
///
/// The notes are picked in a way to maximize the number of notes used,
/// while minimizing the value employed. To do this we sort the notes in
/// ascending value order, and go through each combination in a
/// lexicographic order until we find the first combination whose sum is
/// larger or equal to the given value. If such a slice is not found, an
/// empty vector is returned.
///
/// Note: it is presupposed that the input notes contain enough balance to
/// cover the given `value`.
fn pick_notes(
    value: u64,
    notes_and_values: Vec<(Note, u64, BlsScalar)>,
) -> Vec<(Note, BlsScalar)> {
    let mut notes_and_values = notes_and_values;
    let len = notes_and_values.len();

    if len <= MAX_INPUT_NOTES {
        return notes_and_values
            .into_iter()
            .map(|(note, _, b)| (note, b))
            .collect();
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
    .map(|(n, _, b)| (n, b))
    .to_vec()
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
