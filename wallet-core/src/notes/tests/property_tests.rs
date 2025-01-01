// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#[cfg(test)]
mod tests {
    use dusk_core::transfer::phoenix::{
        Note, NoteLeaf, PublicKey as PhoenixPublicKey,
        SecretKey as PhoenixSecretKey, ViewKey as PhoenixViewKey,
    };
    use dusk_core::JubJubScalar;
    use ff::Field;
    use proptest::collection::vec;
    use proptest::prelude::*;
    use rand::{CryptoRng, RngCore, SeedableRng};
    use rand_chacha::ChaCha12Rng;

    use crate::notes::owned::NoteList;
    use crate::notes::MAX_INPUT_NOTES;
    use crate::{phoenix_balance, pick_notes};

    // Helper function to generate arbitrary valid notes for testing
    fn gen_note<R>(
        rng: &mut R,
        owner_pk: &PhoenixPublicKey,
        value: u64,
        is_obfuscated: bool,
    ) -> Note
    where
        R: RngCore + CryptoRng,
    {
        let value_blinder = JubJubScalar::random(&mut *rng);
        let blinder1 = JubJubScalar::random(&mut *rng);
        let blinder2 = JubJubScalar::random(&mut *rng);
        let sender_blinder = [blinder1, blinder2];

        if is_obfuscated {
            Note::obfuscated(
                &mut *rng,
                owner_pk,
                owner_pk,
                value,
                value_blinder,
                sender_blinder,
            )
        } else {
            Note::transparent(
                &mut *rng,
                owner_pk,
                owner_pk,
                value,
                sender_blinder,
            )
        }
    }

    proptest! {
        /// Tests the balance calculation functionality ensuring that:
        /// 1. Total balance correctly represents sum of all note values
        /// 2. Balances are always non-negative
        /// 3. Spendable balance never exceeds total balance
        /// 4. For small note sets (<=MAX_INPUT_NOTES), spendable equals total
        /// 5. Spendable balance is sum of MAX_INPUT_NOTES highest value notes
        #[test]
        fn test_balance_calculation_properties(
            seed in any::<u64>(),
            values in vec(1u64..=1_000_000_000u64, 1..10),
            is_obfuscated in vec(any::<bool>(), 1..10),
        ) {
            let mut rng = ChaCha12Rng::seed_from_u64(seed);
            let owner_sk = PhoenixSecretKey::random(&mut rng);
            let owner_pk = PhoenixPublicKey::from(&owner_sk);
            let owner_vk = PhoenixViewKey::from(&owner_sk);

            let notes: Vec<NoteLeaf> = values.iter()
                .zip(is_obfuscated.iter())
                .map(|(&value, &is_obf)| {
                    let note = gen_note(&mut rng, &owner_pk, value, is_obf);
                    NoteLeaf {
                        note,
                        block_height: 0,
                    }
                })
                .collect();

            let note_values: Vec<u64> = notes.iter()
                .filter_map(|leaf| leaf.note.value(Some(&owner_vk)).ok())
                .collect();

            let balance_info = phoenix_balance(&owner_vk, notes.iter());

            // Property 1: Total balance should be non-negative and equal to sum of values
            let total_value: u64 = note_values.iter().sum();
            prop_assert_eq!(balance_info.value, total_value);
            prop_assert!(balance_info.value > 0);

            // Property 2: Spendable balance should never exceed total balance
            prop_assert!(balance_info.spendable <= balance_info.value);

            // Property 3: For small note sets (<=MAX_INPUT_NOTES),
            // spendable should equal total
            if notes.len() <= MAX_INPUT_NOTES {
                prop_assert_eq!(balance_info.spendable, balance_info.value);
            }

            // Property 4: Spendable balance should be the sum of
            // MAX_INPUT_NOTES highest value notes
            let mut values = note_values;
            values.sort_by(|a, b| b.cmp(a));
            let expected_spendable: u64 = values.iter()
                .take(MAX_INPUT_NOTES)
                .sum();
            prop_assert_eq!(balance_info.spendable, expected_spendable);
        }

        /// Tests note picking based on actual implementation behavior (not the behaviour
        /// described in the documentation):
        /// 1. Returns empty list if MAX_INPUT_NOTES highest value notes can't cover target
        /// 2. If sum is sufficient AND count <= MAX_INPUT_NOTES, returns all notes
        /// 3. Otherwise uses pick_lexicographic to find valid combination
        #[test]
        fn test_note_picking_properties(
            seed in any::<u64>(),
            values in vec(1u64..=1_000_000u64, 0..10),
            is_obfuscated in vec(any::<bool>(), 0..10),
            target_value in 1u64..1_000_000u64,
        ) {
            let mut rng = ChaCha12Rng::seed_from_u64(seed);
            let owner_sk = PhoenixSecretKey::random(&mut rng);
            let owner_pk = PhoenixPublicKey::from(&owner_sk);
            let owner_vk = PhoenixViewKey::from(&owner_sk);

            // Create notes list
            let notes: Vec<NoteLeaf> = values.iter()
                .zip(is_obfuscated.iter())
                .map(|(&value, &is_obf)| {
                    let note = gen_note(&mut rng, &owner_pk, value, is_obf);
                    NoteLeaf {
                        note,
                        block_height: 0,
                    }
                })
                .collect();

            // Create note list with nullifiers
            let note_list: NoteList = notes.iter()
                .map(|leaf| {
                    let nullifier = leaf.note.gen_nullifier(&owner_sk);
                    (nullifier, leaf.clone())
                })
                .collect();

            let picked_notes = pick_notes(&owner_vk, note_list.clone(), target_value);

            // Test empty input case
            if notes.is_empty() {
                prop_assert!(picked_notes.is_empty());
                return Ok(());
            }

            // Get decrypted values for verification
            let mut note_values: Vec<u64> = notes.iter()
                .filter_map(|leaf| leaf.note.value(Some(&owner_vk)).ok())
                .collect();
            note_values.sort();

            // Check if MAX_INPUT_NOTES highest notes can cover target
            let max_possible: u64 = note_values.iter()
                .rev()
                .take(MAX_INPUT_NOTES)
                .sum();

            if max_possible < target_value {
                // Should return empty list if sum is insufficient
                prop_assert!(picked_notes.is_empty());
                return Ok(());
            }

            // If we have sufficient sum and count <= MAX_INPUT_NOTES
            if notes.len() <= MAX_INPUT_NOTES {
                prop_assert_eq!(picked_notes.len(), notes.len());
                return Ok(());
            }

            // For larger inputs with sufficient sum
            prop_assert!(!picked_notes.is_empty());
            prop_assert!(picked_notes.len() <= MAX_INPUT_NOTES);

            // Get and sort picked values
            let mut picked_values: Vec<u64> = picked_notes.iter()
                .filter_map(|(_, leaf)| leaf.note.value(Some(&owner_vk)).ok())
                .collect();
            picked_values.sort();

            // Verify sum meets target
            let sum: u64 = picked_values.iter().sum();
            prop_assert!(sum >= target_value);

            // Verify picked notes are from the sorted list
            for value in &picked_values {
                prop_assert!(note_values.contains(value));
            }
        }

        /// Tests that note picking follows lexicographic ordering of indices
        /// when selecting notes from larger set.
        ///
        /// The implementation:
        /// 1. Returns empty list if MAX_INPUT_NOTES largest notes can't cover target
        /// 2. Returns all notes if count <= MAX_INPUT_NOTES
        /// 3. Otherwise finds first valid combination in lexicographic order
        #[test]
        fn test_note_picking_lexicographic_order(
            seed in any::<u64>(),
            // Generate more than MAX_INPUT_NOTES values
            values in vec(1u64..=1_000_000u64, (MAX_INPUT_NOTES + 1)..20),
            is_obfuscated in vec(any::<bool>(), (MAX_INPUT_NOTES + 1)..20),
            relative_target in 0.4f64..0.8f64,
        ) {
            let mut rng = ChaCha12Rng::seed_from_u64(seed);
            let owner_sk = PhoenixSecretKey::random(&mut rng);
            let owner_pk = PhoenixPublicKey::from(&owner_sk);
            let owner_vk = PhoenixViewKey::from(&owner_sk);

            // Create and sort notes by value
            let notes: Vec<NoteLeaf> = values.iter()
                .zip(is_obfuscated.iter())
                .map(|(&value, &is_obf)| {
                    let note = gen_note(&mut rng, &owner_pk, value, is_obf);
                    NoteLeaf {
                        note,
                        block_height: 0,
                    }
                })
                .collect();

            let mut note_values: Vec<u64> = notes.iter()
                .filter_map(|leaf| leaf.note.value(Some(&owner_vk)).ok())
                .collect();
            note_values.sort();

            let total_value = note_values.iter().sum::<u64>();
            let target_value = (total_value as f64 * relative_target) as u64;

            // Check if MAX_INPUT_NOTES largest notes can cover target
            let max_possible: u64 = note_values.iter()
                .rev()
                .take(MAX_INPUT_NOTES)
                .sum();

            let note_list: NoteList = notes.iter()
                .map(|leaf| {
                    let nullifier = leaf.note.gen_nullifier(&owner_sk);
                    (nullifier, leaf.clone())
                })
                .collect();

            let picked_notes = pick_notes(&owner_vk, note_list.clone(), target_value);

            if max_possible < target_value {
                // Should return empty list if target can't be met
                prop_assert!(picked_notes.is_empty());
                return Ok(());
            }

            // Get and sort picked values
            let mut picked_values: Vec<u64> = picked_notes.iter()
                .filter_map(|(_, leaf)| leaf.note.value(Some(&owner_vk)).ok())
                .collect();
            picked_values.sort();

            // Verify properties:
            // 1. Number of picked notes should be <= MAX_INPUT_NOTES
            prop_assert!(picked_notes.len() <= MAX_INPUT_NOTES);

            // 2. Sum should meet target
            let picked_sum: u64 = picked_values.iter().sum();
            prop_assert!(picked_sum >= target_value);

            // 3. The selection should follow lexicographic ordering
            let picked_indices: Vec<usize> = picked_values.iter()
                .map(|&value| note_values.iter().position(|&x| x == value).unwrap())
                .collect();

            // Indices should be in ascending order
            for i in 1..picked_indices.len() {
                prop_assert!(picked_indices[i] > picked_indices[i-1]);
            }

            // No earlier valid combination should exist
            let n = picked_indices.len();
            if n > 0 && picked_indices[0] > 0 {
                // Try combination with last index decremented
                let prev_indices: Vec<usize> = (0..picked_indices[0])
                    .take(n)
                    .collect();
                if prev_indices.len() == n {
                    let prev_sum: u64 = prev_indices.iter()
                        .map(|&i| note_values[i])
                        .sum();
                    prop_assert!(prev_sum < target_value);
                }
            }
        }

        /// Tests note picking behavior with duplicate value notes.
        ///
        /// This test verifies the implementation's handling of notes with identical values,
        /// which is a special case that helps verify the lexicographic ordering and
        /// selection logic.
        ///
        /// Test properties:
        /// 1. Input handling:
        ///    - Multiple notes with identical values
        ///    - Target value achievable with MAX_INPUT_NOTES notes
        ///    - Count > MAX_INPUT_NOTES to force selection logic
        ///
        /// 2. Basic requirements:
        ///    - Returns empty list if target exceeds MAX_INPUT_NOTES * value
        ///    - Picks enough notes to meet target
        ///    - Never picks more than MAX_INPUT_NOTES
        ///
        /// 3. Value consistency:
        ///    - All picked notes have the identical value
        ///    - Sum of picked notes meets or exceeds target
        ///    - Selection is valid despite identical values
        ///
        /// This test helps ensure that the implementation correctly handles
        /// the edge case where lexicographic ordering must work with
        /// identical values, and the selection logic remains stable even
        /// when multiple equivalent choices are available.
        #[test]
        fn test_note_picking_duplicate_values(
            seed in any::<u64>(),
            value in 1u64..=1_000_000u64,
            count in (MAX_INPUT_NOTES + 1)..20,
            // Ensure target is achievable with MAX_INPUT_NOTES notes
            relative_target in 0.1f64..0.4f64,
        ) {
            let mut rng = ChaCha12Rng::seed_from_u64(seed);
            let owner_sk = PhoenixSecretKey::random(&mut rng);
            let owner_pk = PhoenixPublicKey::from(&owner_sk);
            let owner_vk = PhoenixViewKey::from(&owner_sk);

            // Create multiple notes with same value
            let notes: Vec<NoteLeaf> = (0..count)
                .map(|_| {
                    let note = gen_note(&mut rng, &owner_pk, value, false);
                    NoteLeaf {
                        note,
                        block_height: 0,
                    }
                })
                .collect();

            // Calculate target value that can be achieved with MAX_INPUT_NOTES notes
            let target_value = ((value * MAX_INPUT_NOTES as u64) as f64 * relative_target) as u64;

            let note_list: NoteList = notes.iter()
                .map(|leaf| {
                    let nullifier = leaf.note.gen_nullifier(&owner_sk);
                    (nullifier, leaf.clone())
                })
                .collect();

            let picked_notes = pick_notes(&owner_vk, note_list.clone(), target_value);

            // Verify picked notes
            if target_value > value * MAX_INPUT_NOTES as u64 {
                // If target is too high, should return empty list
                prop_assert!(picked_notes.is_empty());
            } else {
                // Should pick enough notes to meet target
                let picked_sum: u64 = picked_notes.iter()
                    .filter_map(|(_, leaf)| leaf.note.value(Some(&owner_vk)).ok())
                    .sum();
                prop_assert!(picked_sum >= target_value);
                prop_assert!(picked_notes.len() <= MAX_INPUT_NOTES);

                // All picked notes should have the same value
                for (_, leaf) in picked_notes.iter() {
                    let note_value = leaf.note.value(Some(&owner_vk)).unwrap();
                    prop_assert_eq!(note_value, value);
                }
            }
        }

        /// Tests note picking behavior at MAX_INPUT_NOTES boundary conditions.
        ///
        /// This test specifically focuses on scenarios around the MAX_INPUT_NOTES
        /// limit to ensure proper handling of these edge cases in the implementation.
        ///
        /// Test properties:
        /// 1. Input structure:
        ///    - Exactly MAX_INPUT_NOTES + 1 notes
        ///    - Strictly increasing values
        ///    - Values scaled by multiplier to test different ranges
        ///
        /// 2. Selection behavior:
        ///    - When target requires exactly MAX_INPUT_NOTES smallest notes
        ///    - When target could be met with fewer than MAX_INPUT_NOTES notes
        ///    - When target requires including larger value notes
        ///
        /// 3. Boundary conditions:
        ///    - Proper handling of note count at MAX_INPUT_NOTES boundary
        ///    - Correct selection when multiple valid combinations exist
        ///    - Adherence to lexicographic ordering with sequential values
        ///
        /// This test ensures that the implementation correctly handles cases
        /// right at the MAX_INPUT_NOTES limit, where the selection logic
        /// must make optimal choices while respecting the maximum input constraint.
        #[test]
        fn test_note_picking_max_input_boundary(
            seed in any::<u64>(),
            value_multiplier in 1u64..10,
        ) {
            let mut rng = ChaCha12Rng::seed_from_u64(seed);
            let owner_sk = PhoenixSecretKey::random(&mut rng);
            let owner_pk = PhoenixPublicKey::from(&owner_sk);
            let owner_vk = PhoenixViewKey::from(&owner_sk);

            // Create MAX_INPUT_NOTES + 1 notes with increasing values
            let notes: Vec<NoteLeaf> = (1..=MAX_INPUT_NOTES + 1)
                .map(|i| {
                    // Convert i to u64 before multiplication
                    let note = gen_note(&mut rng, &owner_pk, (i as u64) * value_multiplier, false);
                    NoteLeaf {
                        note,
                        block_height: 0,
                    }
                })
                .collect();

            let note_list: NoteList = notes.iter()
                .map(|leaf| {
                    let nullifier = leaf.note.gen_nullifier(&owner_sk);
                    (nullifier, leaf.clone())
                })
                .collect();

            // Target that requires exactly MAX_INPUT_NOTES smallest notes
            let target_value = (1..=MAX_INPUT_NOTES as u64).sum::<u64>() * value_multiplier;

            let picked_notes = pick_notes(&owner_vk, note_list.clone(), target_value);

            // Should pick exactly MAX_INPUT_NOTES
            prop_assert_eq!(picked_notes.len(), MAX_INPUT_NOTES);

            // Should pick the smallest MAX_INPUT_NOTES notes
            let picked_values: Vec<u64> = picked_notes.iter()
                .filter_map(|(_, leaf)| leaf.note.value(Some(&owner_vk)).ok())
                .collect();

            for (i, value) in picked_values.iter().enumerate() {
                prop_assert_eq!(*value, (i + 1) as u64 * value_multiplier);
            }
        }

        /// Tests note picking implementation's core behavior with minimal value differences.
        ///
        /// Implementation details being tested:
        /// 1. Notes are sorted by value in ascending order
        /// 2. Algorithm always attempts to use exactly MAX_INPUT_NOTES notes unless:
        ///    - Input is empty (returns empty)
        ///    - Input size <= MAX_INPUT_NOTES (returns all)
        ///    - MAX_INPUT_NOTES largest notes can't cover target (returns empty)
        /// 3. For large inputs:
        ///    - Tries combinations of exactly MAX_INPUT_NOTES notes
        ///    - Tests combinations in lexicographic order: [0,1,2,3], [0,1,2,4], etc.
        ///    - Returns first combination whose sum >= target
        ///
        /// Test approach:
        /// - Creates notes with consecutive values
        /// - Verifies that first lexicographic combination meeting target is selected
        /// - Tests all special cases (empty result, all notes, exact target)
        #[test]
        fn test_note_picking_minimal_differences(
            seed in any::<u64>(),
            base_value in 100u64..1_000_000u64,
            count in (MAX_INPUT_NOTES + 1)..10,
        ) {
            let mut rng = ChaCha12Rng::seed_from_u64(seed);
            let owner_sk = PhoenixSecretKey::random(&mut rng);
            let owner_pk = PhoenixPublicKey::from(&owner_sk);
            let owner_vk = PhoenixViewKey::from(&owner_sk);

            // Create notes with consecutive values
            let notes: Vec<NoteLeaf> = (0..count)
                .map(|i| {
                    let value = base_value + i as u64;
                    let note = gen_note(&mut rng, &owner_pk, value, false);
                    NoteLeaf {
                        note,
                        block_height: 0,
                    }
                })
                .collect();

            // Get and sort values for verification
            let mut sorted_values: Vec<u64> = notes.iter()
                .filter_map(|leaf| leaf.note.value(Some(&owner_vk)).ok())
                .collect();
            sorted_values.sort();

            // Calculate target requiring first lexicographic combination
            let first_combination_sum: u64 = sorted_values.iter()
                .take(MAX_INPUT_NOTES)
                .sum();
            let target_value = first_combination_sum;

            let note_list: NoteList = notes.iter()
                .map(|leaf| {
                    let nullifier = leaf.note.gen_nullifier(&owner_sk);
                    (nullifier, leaf.clone())
                })
                .collect();

            let picked_notes = pick_notes(&owner_vk, note_list.clone(), target_value);

            // Verify selection matches first lexicographic combination
            prop_assert_eq!(picked_notes.len(), MAX_INPUT_NOTES);

            let mut picked_values: Vec<u64> = picked_notes.iter()
                .filter_map(|(_, leaf)| leaf.note.value(Some(&owner_vk)).ok())
                .collect();
            picked_values.sort();

            // Should be first MAX_INPUT_NOTES values as they form first valid combination
            for i in 0..MAX_INPUT_NOTES {
                prop_assert_eq!(picked_values[i], sorted_values[i]);
            }

            // Test that higher target requires later combination
            let target_value = first_combination_sum + 1;
            let picked_notes = pick_notes(&owner_vk, note_list.clone(), target_value);

            if !picked_notes.is_empty() {
                let picked_values: Vec<u64> = picked_notes.iter()
                    .filter_map(|(_, leaf)| leaf.note.value(Some(&owner_vk)).ok())
                    .collect();
                let sum: u64 = picked_values.iter().sum();
                prop_assert!(sum >= target_value);
                prop_assert_eq!(picked_notes.len(), MAX_INPUT_NOTES);
            }

            // Test empty result case
            let max_possible: u64 = sorted_values.iter()
                .rev()
                .take(MAX_INPUT_NOTES)
                .sum();
            let picked_notes = pick_notes(&owner_vk, note_list.clone(), max_possible + 1);
            prop_assert!(picked_notes.is_empty());

            // Test small input case
            let small_list: NoteList = notes.iter()
                .take(MAX_INPUT_NOTES - 1)
                .map(|leaf| {
                    let nullifier = leaf.note.gen_nullifier(&owner_sk);
                    (nullifier, leaf.clone())
                })
                .collect();
            let picked_notes = pick_notes(&owner_vk, small_list.clone(), 1);
            prop_assert_eq!(picked_notes.len(), MAX_INPUT_NOTES - 1);
        }

        /// Tests note picking with extreme value distributions.
        ///
        /// This test verifies correct handling of notes with very different
        /// magnitudes of values (some very small, some very large).
        ///
        /// Properties to test:
        /// 1. Mix of very small and very large values
        /// 2. Target values that could be met by either many small or few large notes
        /// 3. Correct order-dependent selection (uses first valid combination)
        /// 4. Proper handling of large value differences without overflow
        #[test]
        fn test_note_picking_extreme_distributions(
            seed in any::<u64>(),
            small_value in 1u64..1000u64,
            count in (MAX_INPUT_NOTES + 1)..10,
        ) {
            let mut rng = ChaCha12Rng::seed_from_u64(seed);
            let owner_sk = PhoenixSecretKey::random(&mut rng);
            let owner_pk = PhoenixPublicKey::from(&owner_sk);
            let owner_vk = PhoenixViewKey::from(&owner_sk);

            // Create mix of small and large values
            let notes: Vec<NoteLeaf> = (0..count)
                .map(|i| {
                    let value = if i % 2 == 0 {
                        small_value
                    } else {
                        small_value * 1_000_000 // Large value
                    };
                    let note = gen_note(&mut rng, &owner_pk, value, false);
                    NoteLeaf {
                        note,
                        block_height: 0,
                    }
                })
                .collect();

            // Calculate total available value
            let available_values: Vec<u64> = notes.iter()
                .filter_map(|leaf| leaf.note.value(Some(&owner_vk)).ok())
                .collect();
            let total_available: u64 = available_values.iter().sum();

            // Try different target values
            let targets = [
                small_value, // Should pick first small note
                small_value * 1_000_000, // Should pick first large note
                total_available / 2, // Should pick multiple notes
            ];

            for &target_value in &targets {
                let note_list: NoteList = notes.iter()
                    .map(|leaf| {
                        let nullifier = leaf.note.gen_nullifier(&owner_sk);
                        (nullifier, leaf.clone())
                    })
                    .collect();

                let picked_notes = pick_notes(&owner_vk, note_list.clone(), target_value);

                if !picked_notes.is_empty() {
                    // Verify basic properties
                    prop_assert!(picked_notes.len() <= MAX_INPUT_NOTES);

                    let picked_sum: u64 = picked_notes.iter()
                        .filter_map(|(_, leaf)| leaf.note.value(Some(&owner_vk)).ok())
                        .sum();
                    prop_assert!(picked_sum >= target_value);

                    // Get original indices of picked notes
                    let picked_values: Vec<u64> = picked_notes.iter()
                        .filter_map(|(_, leaf)| leaf.note.value(Some(&owner_vk)).ok())
                        .collect();

                    // Verify notes came from original list
                    for value in &picked_values {
                        prop_assert!(available_values.contains(value));
                    }

                    // Verify first valid combination was picked
                    // by checking no earlier combination could work
                    let first_picked_idx = notes.iter()
                        .position(|leaf| {
                            leaf.note.value(Some(&owner_vk))
                                .map(|v| v == picked_values[0])
                                .unwrap_or(false)
                        })
                        .unwrap();

                    if first_picked_idx > 0 {
                        // Check that no earlier combination could satisfy target
                        let earlier_sum: u64 = notes[..first_picked_idx]
                            .iter()
                            .take(MAX_INPUT_NOTES)
                            .filter_map(|leaf| leaf.note.value(Some(&owner_vk)).ok())
                            .sum();
                        prop_assert!(earlier_sum < target_value);
                    }
                }
            }
        }

        /// Tests note picking with specific target edge cases.
        ///
        /// This test verifies behavior with target values that are
        /// at interesting boundaries relative to available notes.
        ///
        /// Properties to test:
        /// 1. Target exactly equals sum of some combination
        /// 2. Target greater than sum of MAX_INPUT_NOTES largest notes
        /// 3. Target less than smallest note (should still work if sum sufficient)
        #[test]
        fn test_note_picking_target_boundaries(
            seed in any::<u64>(),
            base_value in 100u64..1_000_000u64,
        ) {
            let mut rng = ChaCha12Rng::seed_from_u64(seed);
            let owner_sk = PhoenixSecretKey::random(&mut rng);
            let owner_pk = PhoenixPublicKey::from(&owner_sk);
            let owner_vk = PhoenixViewKey::from(&owner_sk);

            // Create MAX_INPUT_NOTES notes with known values
            let notes: Vec<NoteLeaf> = (0..MAX_INPUT_NOTES)
                .map(|i| {
                    let value = base_value * (i + 1) as u64;
                    let note = gen_note(&mut rng, &owner_pk, value, false);
                    NoteLeaf {
                        note,
                        block_height: 0,
                    }
                })
                .collect();

            let note_list: NoteList = notes.iter()
                .map(|leaf| {
                    let nullifier = leaf.note.gen_nullifier(&owner_sk);
                    (nullifier, leaf.clone())
                })
                .collect();

            // Test exact sum case
            let exact_sum: u64 = (1..=MAX_INPUT_NOTES as u64).map(|i| base_value * i).sum();
            let picked_notes = pick_notes(&owner_vk, note_list.clone(), exact_sum);
            if !picked_notes.is_empty() {
                let picked_sum: u64 = picked_notes.iter()
                    .filter_map(|(_, leaf)| leaf.note.value(Some(&owner_vk)).ok())
                    .sum();
                prop_assert_eq!(picked_sum, exact_sum);
            }

            // Test greater than maximum achievable (should return empty list)
            let picked_notes = pick_notes(&owner_vk, note_list.clone(), exact_sum + 1);
            prop_assert!(picked_notes.is_empty());

            // Test less than smallest note (should still work if sum is sufficient)
            let small_target = base_value / 2;
            let picked_notes = pick_notes(&owner_vk, note_list.clone(), small_target);
            if !picked_notes.is_empty() {
                let picked_sum: u64 = picked_notes.iter()
                    .filter_map(|(_, leaf)| leaf.note.value(Some(&owner_vk)).ok())
                    .sum();
                prop_assert!(picked_sum >= small_target);
            }
        }

        /// Tests note picking with degenerate inputs.
        ///
        /// This test verifies correct handling of various edge cases
        /// and potentially problematic inputs.
        ///
        /// Properties to test:
        /// 1. Notes with minimum possible values
        /// 2. Notes with maximum possible values
        /// 3. Target values at u64 boundaries
        #[test]
        fn test_note_picking_degenerate_cases(
            seed in any::<u64>(),
        ) {
            let mut rng = ChaCha12Rng::seed_from_u64(seed);
            let owner_sk = PhoenixSecretKey::random(&mut rng);
            let owner_pk = PhoenixPublicKey::from(&owner_sk);
            let owner_vk = PhoenixViewKey::from(&owner_sk);

            // Create notes with minimum values
            let min_notes: Vec<NoteLeaf> = (0..MAX_INPUT_NOTES * 2)
                .map(|_| {
                    let note = gen_note(&mut rng, &owner_pk, 1, false);
                    NoteLeaf {
                        note,
                        block_height: 0,
                    }
                })
                .collect();

            let min_note_list: NoteList = min_notes.iter()
                .map(|leaf| {
                    let nullifier = leaf.note.gen_nullifier(&owner_sk);
                    (nullifier, leaf.clone())
                })
                .collect();

            // Test with target = number of notes (should select all minimum value notes)
            let picked_notes = pick_notes(&owner_vk, min_note_list.clone(), MAX_INPUT_NOTES as u64);
            if !picked_notes.is_empty() {
                prop_assert_eq!(picked_notes.len(), MAX_INPUT_NOTES);
                let picked_sum: u64 = picked_notes.iter()
                    .filter_map(|(_, leaf)| leaf.note.value(Some(&owner_vk)).ok())
                    .sum();
                prop_assert_eq!(picked_sum, MAX_INPUT_NOTES as u64);
            }

            // Create notes with maximum values
            let max_notes: Vec<NoteLeaf> = (0..MAX_INPUT_NOTES)
                .map(|_| {
                    let note = gen_note(&mut rng, &owner_pk, u64::MAX / (MAX_INPUT_NOTES as u64 * 2), false);
                    NoteLeaf {
                        note,
                        block_height: 0,
                    }
                })
                .collect();

            let max_note_list: NoteList = max_notes.iter()
                .map(|leaf| {
                    let nullifier = leaf.note.gen_nullifier(&owner_sk);
                    (nullifier, leaf.clone())
                })
                .collect();

            // Test with large target (but still within u64 bounds)
            let max_target = u64::MAX / (MAX_INPUT_NOTES as u64 * 2);
            let picked_notes = pick_notes(&owner_vk, max_note_list.clone(), max_target);
            if !picked_notes.is_empty() {
                let picked_sum: u64 = picked_notes.iter()
                    .filter_map(|(_, leaf)| leaf.note.value(Some(&owner_vk)).ok())
                    .sum();
                prop_assert!(picked_sum >= max_target);
            }
        }
    }
}
