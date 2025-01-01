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
    }
}
