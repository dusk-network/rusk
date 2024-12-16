// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Helper functions for working with notes.

use alloc::vec::Vec;

use dusk_core::transfer::phoenix::{NoteLeaf, ViewKey as PhoenixViewKey};
use dusk_core::BlsScalar;

use crate::notes::owned::NoteList;
use crate::notes::MAX_INPUT_NOTES;

/// Pick up to [`MAX_INPUT_NOTES`] notes to be used as input-notes in a
/// transaction from a list of owned notes.
///
/// The notes are picked in a way to maximize the number of notes used, while
/// minimizing the value employed. To do this we sort the notes in ascending
/// value order, and go through each combination in a lexicographic order until
/// we find the first combination whose sum is larger or equal to the given
/// cost.
///
/// If the cost is greater than the sum of the [`MAX_INPUT_NOTES`] notes with
/// the largest value, no combination can be found and an empty vector is
/// returned.
#[must_use]
pub fn notes(vk: &PhoenixViewKey, notes: NoteList, cost: u64) -> NoteList {
    if notes.is_empty() {
        return NoteList::default();
    }

    // decrypt the note-values
    let mut notes_values_nullifier: Vec<(NoteLeaf, u64, BlsScalar)> = notes
        .iter()
        .filter_map(|(nullifier, leaf)| {
            leaf.as_ref()
                .value(Some(vk))
                .ok()
                .map(|value| (leaf.clone(), value, *nullifier))
        })
        .collect();

    // sort the input-notes from smallest to largest value
    notes_values_nullifier.sort_by(|(_, aval, _), (_, bval, _)| aval.cmp(bval));

    // return an empty list if the MAX_INPUT_NOTES highest notes do not cover
    // the cost
    if notes_values_nullifier
        .iter()
        .rev()
        .take(MAX_INPUT_NOTES)
        .map(|notes_values_nullifier| notes_values_nullifier.1)
        .sum::<u64>()
        < cost
    {
        return NoteList::default();
    }

    // if there are less than MAX_INPUT_NOTES notes, we can return the list as
    // it is
    if notes.len() <= MAX_INPUT_NOTES {
        return notes;
    }

    // at this point we know that there is a possible combination of
    // MAX_INPUT_NOTES notes that cover the cost and we pick the combination of
    // MAX_INPUT_NOTES notes with the smallest possible values whose sum cover
    // the cost
    pick_lexicographic(&notes_values_nullifier, cost)
        .map(|index| notes_values_nullifier[index].clone())
        .map(|(n, _, b)| (b, n))
        .to_vec()
        .into()
}

// Sum up the values of the MAX_INPUT_NOTES notes stored at the given indices
// and check that this sum is larger or equal the given cost.
fn is_valid(
    notes_values_nullifier: impl AsRef<[(NoteLeaf, u64, BlsScalar)]>,
    cost: u64,
    indices: &[usize; MAX_INPUT_NOTES],
) -> bool {
    indices
        .iter()
        .map(|index| notes_values_nullifier.as_ref()[*index].1)
        .sum::<u64>()
        >= cost
}

// Pick a combination of exactly MAX_INPUT_NOTES notes that cover the cost.
// Notes with smaller values are favored.
fn pick_lexicographic(
    notes_values_nullifier: &Vec<(NoteLeaf, u64, BlsScalar)>,
    cost: u64,
) -> [usize; MAX_INPUT_NOTES] {
    let max_len = notes_values_nullifier.len();

    // initialize the indices
    let mut indices = [0; MAX_INPUT_NOTES];
    indices
        .iter_mut()
        .enumerate()
        .for_each(|(i, index)| *index = i);

    loop {
        if is_valid(notes_values_nullifier, cost, &indices) {
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

    indices
}
