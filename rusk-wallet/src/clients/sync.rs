// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashSet;
use std::future::Future;

use dusk_bytes::Serializable;
use dusk_core::BlsScalar;
use dusk_core::transfer::TRANSFER_CONTRACT;
use dusk_core::transfer::phoenix::{
    Note, NoteLeaf, PublicKey as PhoenixPublicKey,
    SecretKey as PhoenixSecretKey, ViewKey as PhoenixViewKey,
};
use futures::StreamExt;
use rayon::ThreadPoolBuilder;
use rayon::prelude::*;
use rkyv::Deserialize;
use wallet_core::keys::{
    derive_phoenix_pk, derive_phoenix_sk, derive_phoenix_vk,
};
use zeroize::Zeroize;

use super::{LocalStore, MAX_PROFILES, TREE_LEAF};
use crate::Error;
use crate::clients::Cache;
use crate::rues::{CONTRACTS_TARGET, HttpClient as RuesHttpClient};

const SYNC_PROGRESS_BLOCK_STEP: u64 = 100;
const DEFAULT_OWNERSHIP_CHUNK_SIZE: usize = 512;
const DEFAULT_NULLIFIER_BATCH_BYTE_BUDGET: usize = 60 * 1024;
const NULLIFIER_BATCH_LAYOUT_BYTES: usize = u64::SIZE;

#[derive(Clone, Debug, PartialEq)]
struct OwnedNote {
    block_height: u64,
    note: Note,
}

pub(crate) async fn sync_db(
    client: &RuesHttpClient,
    cache: &Cache,
    store: &LocalStore,
    status: fn(&str),
    allow_cache_reset: bool,
) -> Result<(), Error> {
    let seed = store.get_seed();

    let mut keys: Vec<(PhoenixSecretKey, PhoenixViewKey, PhoenixPublicKey)> =
        (0..MAX_PROFILES)
            .map(|i| {
                // we know that `i < MAX_PROFILES <= u8::MAX`, so casting to u8
                // is safe here
                #[allow(clippy::cast_possible_truncation)]
                let i = i as u8;
                (
                    derive_phoenix_sk(seed, i),
                    derive_phoenix_vk(seed, i),
                    derive_phoenix_pk(seed, i),
                )
            })
            .collect();

    status("Getting cached note position...");

    let last_pos = cache.last_pos().inspect_err(|_| {
        zeroize_secret_keys(&mut keys);
    })?;
    let mut pos_to_search = last_pos.map(|p| p + 1).unwrap_or_default();

    // Re-fetch a known owned note and verify its content matches the chain.
    if pos_to_search > 0 {
        let pks: Vec<PhoenixPublicKey> =
            keys.iter().map(|(_, _, pk)| *pk).collect();
        if check_stale_cache(
            cache,
            &pks,
            |pos| fetch_note_at_pos(client, pos),
            allow_cache_reset,
            status,
        )
        .await
        .inspect_err(|_| zeroize_secret_keys(&mut keys))?
        {
            pos_to_search = 0;
        }
    }

    if pos_to_search > 0 {
        status(&format!(
            "Resuming sync from cached note position {pos_to_search}"
        ));
    }

    let (last_pos, max_block_height, note_data) =
        collect_fresh_notes(client, pos_to_search, &mut keys, status).await?;

    let owned_notes = collect_owned_notes(&keys, note_data.as_slice());
    persist_owned_notes(client, cache, &keys, &owned_notes).await?;

    zeroize_secret_keys(&mut keys);

    // Remove spent nullifiers from live notes
    reconcile_cached_unspent_notes(client, cache, &keys).await?;

    // insert last post after the notes has been inserted
    // to prevent false reporting of sync completion
    cache.insert_last_pos(last_pos)?;
    status(&format!(
        "Syncing Complete at block {max_block_height} (note position {last_pos})"
    ));

    Ok(())
}

async fn collect_fresh_notes(
    client: &RuesHttpClient,
    pos_to_search: u64,
    keys: &mut [(PhoenixSecretKey, PhoenixViewKey, PhoenixPublicKey)],
    status: fn(&str),
) -> Result<(u64, u64, Vec<(u64, Note)>), Error> {
    status("Fetching fresh notes...");

    let req = rkyv::to_bytes::<_, 8>(&(pos_to_search))
        .map_err(|_| Error::Rkyv)?
        .to_vec();

    let mut stream = client
        .call_contract_raw(
            CONTRACTS_TARGET,
            TRANSFER_CONTRACT,
            "leaves_from_pos",
            &req,
            true,
        )
        .await
        .inspect_err(|_| zeroize_secret_keys(keys))?
        .bytes_stream();

    status("Connection established...");
    status("Streaming notes...");

    let mut last_pos = pos_to_search.saturating_sub(1);
    let mut max_block_height = 0_u64;
    let mut last_reported_block = 0_u64;

    // This buffer is needed because `.bytes_stream();` introduces additional
    // splitting of chunks according to its own buffer.
    let mut buffer = vec![];
    let mut note_data = Vec::new();

    while let Some(http_chunk) = stream.next().await {
        buffer.extend_from_slice(
            &http_chunk.inspect_err(|_| zeroize_secret_keys(keys))?,
        );

        let mut leaf_chunk = buffer.chunks_exact(TREE_LEAF);
        for leaf_bytes in leaf_chunk.by_ref() {
            let NoteLeaf { block_height, note } =
                rkyv::check_archived_root::<NoteLeaf>(leaf_bytes)
                    .map_err(|_| Error::Rkyv)
                    .inspect_err(|_| zeroize_secret_keys(keys))?
                    .deserialize(&mut rkyv::Infallible)
                    .unwrap();

            last_pos = std::cmp::max(last_pos, *note.pos());
            max_block_height = std::cmp::max(max_block_height, block_height);
            note_data.push((block_height, note));

            if max_block_height
                >= last_reported_block + SYNC_PROGRESS_BLOCK_STEP
            {
                status(&format!(
                    "Syncing chain state at block {max_block_height}"
                ));
                last_reported_block = max_block_height;
            }
        }

        buffer = leaf_chunk.remainder().to_vec();
    }

    Ok((last_pos, max_block_height, note_data))
}

fn collect_owned_notes(
    keys: &[(PhoenixSecretKey, PhoenixViewKey, PhoenixPublicKey)],
    note_data: &[(u64, Note)],
) -> Vec<Vec<OwnedNote>> {
    if note_data.len() <= DEFAULT_OWNERSHIP_CHUNK_SIZE {
        return collect_owned_notes_serial(keys, note_data);
    }

    let thread_pool = ThreadPoolBuilder::new()
        .num_threads(default_ownership_workers())
        .build();

    let Ok(thread_pool) = thread_pool else {
        return collect_owned_notes_serial(keys, note_data);
    };

    let chunk_results = thread_pool.install(|| {
        note_data
            .par_chunks(DEFAULT_OWNERSHIP_CHUNK_SIZE)
            .map(|chunk| collect_owned_notes_serial(keys, chunk))
            .collect::<Vec<_>>()
    });

    let mut owned = vec![Vec::new(); keys.len()];
    for mut chunk_owned in chunk_results {
        for (notes, chunk_notes) in owned.iter_mut().zip(chunk_owned.iter_mut())
        {
            notes.append(chunk_notes);
        }
    }

    owned
}

fn collect_owned_notes_serial(
    keys: &[(PhoenixSecretKey, PhoenixViewKey, PhoenixPublicKey)],
    note_data: &[(u64, Note)],
) -> Vec<Vec<OwnedNote>> {
    let mut owned = vec![Vec::new(); keys.len()];

    for (block_height, note) in note_data {
        for (profile_idx, (_, vk, _)) in keys.iter().enumerate() {
            if vk.owns(note.stealth_address()) {
                owned[profile_idx].push(OwnedNote {
                    block_height: *block_height,
                    note: note.clone(),
                });
                break;
            }
        }
    }

    owned
}

async fn persist_owned_notes(
    client: &RuesHttpClient,
    cache: &Cache,
    keys: &[(PhoenixSecretKey, PhoenixViewKey, PhoenixPublicKey)],
    owned_notes: &[Vec<OwnedNote>],
) -> Result<(), Error> {
    let batch_len = nullifier_batch_len();

    for ((sk, _, pk), profile_notes) in keys.iter().zip(owned_notes) {
        let pk_bs58 = bs58::encode(pk.to_bytes()).into_string();

        for notes_batch in profile_notes.chunks(batch_len) {
            let nullifiers = notes_batch
                .iter()
                .map(|owned_note| owned_note.note.gen_nullifier(sk))
                .collect::<Vec<_>>();
            let spent_nullifiers =
                fetch_existing_nullifiers_remote(client, nullifiers.as_slice())
                    .await?;
            let spent_lookup = spent_nullifiers
                .into_iter()
                .map(|nullifier| nullifier.to_bytes())
                .collect::<HashSet<_>>();

            for (owned_note, nullifier) in notes_batch.iter().zip(nullifiers) {
                let note = (owned_note.note.clone(), nullifier);
                if spent_lookup.contains(&note.1.to_bytes()) {
                    cache.insert_spent(
                        &pk_bs58,
                        owned_note.block_height,
                        note,
                    )?;
                } else {
                    cache.insert(&pk_bs58, owned_note.block_height, note)?;
                }
            }
        }
    }

    Ok(())
}

async fn reconcile_cached_unspent_notes(
    client: &RuesHttpClient,
    cache: &Cache,
    keys: &[(PhoenixSecretKey, PhoenixViewKey, PhoenixPublicKey)],
) -> Result<(), Error> {
    let batch_len = nullifier_batch_len();

    for (_, _, pk) in keys {
        let nullifiers: Vec<BlsScalar> = cache.unspent_notes_id(pk)?;

        if nullifiers.is_empty() {
            continue;
        }

        for nullifier_batch in nullifiers.chunks(batch_len) {
            let existing =
                fetch_existing_nullifiers_remote(client, nullifier_batch)
                    .await?;
            cache.spend_notes(pk, existing.as_slice())?;
        }
    }

    Ok(())
}

fn default_ownership_workers() -> usize {
    std::thread::available_parallelism()
        .map(std::num::NonZeroUsize::get)
        .unwrap_or(1)
        .max(1)
}

fn nullifier_batch_len() -> usize {
    DEFAULT_NULLIFIER_BATCH_BYTE_BUDGET
        .saturating_sub(NULLIFIER_BATCH_LAYOUT_BYTES)
        .checked_div(BlsScalar::SIZE)
        .unwrap_or_default()
        .max(1)
}

fn zeroize_secret_keys(
    keys: &mut [(PhoenixSecretKey, PhoenixViewKey, PhoenixPublicKey)],
) {
    for (sk, _, _) in keys.iter_mut() {
        sk.zeroize();
    }
}

/// Checks whether the cache is stale by comparing an owned note against the
/// live chain. Returns `true` if the cache was stale and has been reset.
async fn check_stale_cache<F, Fut>(
    cache: &Cache,
    pks: &[PhoenixPublicKey],
    fetch_note: F,
    allow_cache_reset: bool,
    status: fn(&str),
) -> Result<bool, Error>
where
    F: Fn(u64) -> Fut,
    Fut: Future<Output = Result<Option<Note>, Error>>,
{
    if let Some(anchor) = cache.note_for_cache_validation(pks)? {
        let note_pos = *anchor.pos();
        let chain_note = fetch_note(note_pos).await?;
        if chain_note.as_ref() != Some(&anchor) {
            if !allow_cache_reset {
                return Err(Error::StaleCache(note_pos));
            }
            tracing::warn!(
                note_pos,
                "cached note mismatch — resetting stale cache"
            );
            status("Stale cache detected — resetting note cache...");
            cache.clear_notes(pks)?;
            return Ok(true);
        }
    }
    Ok(false)
}

/// Fetches the note at `pos` from the note tree, or `None` if absent.
async fn fetch_note_at_pos(
    client: &RuesHttpClient,
    pos: u64,
) -> Result<Option<Note>, Error> {
    let req = rkyv::to_bytes::<_, 16>(&(pos, 1u64))
        .map_err(|_| Error::Rkyv)?
        .to_vec();
    let mut stream = client
        .call_contract_raw(
            CONTRACTS_TARGET,
            TRANSFER_CONTRACT,
            "sync",
            &req,
            true,
        )
        .await?
        .bytes_stream();
    let mut buffer = vec![];
    while let Some(http_chunk) = stream.next().await {
        buffer.extend_from_slice(&http_chunk?);
        if buffer.len() >= TREE_LEAF {
            let NoteLeaf { note, .. } =
                rkyv::check_archived_root::<NoteLeaf>(&buffer[..TREE_LEAF])
                    .map_err(|_| Error::Rkyv)?
                    .deserialize(&mut rkyv::Infallible)
                    .unwrap();
            return Ok((*note.pos() == pos).then_some(note));
        }
    }
    Ok(None)
}

/// Asks the node to return the nullifiers that already exist from the given
/// nullifiers.
pub(crate) async fn fetch_existing_nullifiers_remote(
    client: &RuesHttpClient,
    nullifiers: &[BlsScalar],
) -> Result<Vec<BlsScalar>, Error> {
    if nullifiers.is_empty() {
        return Ok(vec![]);
    }
    let nullifiers = nullifiers.to_vec();
    let data = client
        .contract_query::<_, 1024>(
            TRANSFER_CONTRACT,
            "existing_nullifiers",
            &nullifiers,
        )
        .await?;

    let nullifiers = rkyv::check_archived_root::<Vec<BlsScalar>>(&data)
        .map_err(|_| Error::Rkyv)?
        .deserialize(&mut rkyv::Infallible)
        .unwrap();

    Ok(nullifiers)
}

#[cfg(test)]
mod tests {
    use dusk_core::transfer::phoenix::{
        Note, PublicKey as PhoenixPublicKey, SecretKey as PhoenixSecretKey,
    };
    use dusk_core::{BlsScalar, JubJubScalar};
    use ff::Field;
    use rand::rngs::StdRng;
    use rand::{CryptoRng, RngCore, SeedableRng};

    use super::*;
    use crate::Error;
    use crate::clients::Cache;

    fn gen_note<T: RngCore + CryptoRng>(
        rng: &mut T,
        owner_pk: &PhoenixPublicKey,
        value: u64,
    ) -> Note {
        let sender_pk = PhoenixPublicKey::from(&PhoenixSecretKey::random(rng));
        let value_blinder = JubJubScalar::random(&mut *rng);
        let sender_blinder = [
            JubJubScalar::random(&mut *rng),
            JubJubScalar::random(&mut *rng),
        ];

        Note::obfuscated(
            rng,
            &sender_pk,
            owner_pk,
            value,
            value_blinder,
            sender_blinder,
        )
    }

    #[test]
    fn parallel_owned_scan_matches_serial() {
        let mut rng = StdRng::seed_from_u64(0xdab);
        let seed = [7; 64];
        let keys: Vec<_> = (0..4u8)
            .map(|index| {
                (
                    derive_phoenix_sk(&seed, index),
                    derive_phoenix_vk(&seed, index),
                    derive_phoenix_pk(&seed, index),
                )
            })
            .collect();

        let note_data = (0..2_048)
            .map(|idx| {
                let owner_idx = idx % (keys.len() + 1);
                let fallback_owner =
                    PhoenixPublicKey::from(&PhoenixSecretKey::random(&mut rng));
                let owner_pk = keys
                    .get(owner_idx)
                    .map_or(&fallback_owner, |(_, _, pk)| pk);

                (idx as u64, gen_note(&mut rng, owner_pk, idx as u64 + 1))
            })
            .collect::<Vec<_>>();

        let serial = collect_owned_notes_serial(&keys, &note_data);
        let parallel = collect_owned_notes(&keys, &note_data);

        assert_eq!(serial, parallel);
    }

    fn cache_with_note(
        pk: &PhoenixPublicKey,
        note: Note,
    ) -> (Cache, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let bs58_repr = bs58::encode(pk.to_bytes()).into_string();
        let cfs = vec![bs58_repr.clone(), format!("spent_{bs58_repr}")];
        let cache = Cache::new(dir.path(), cfs, |_| {}).unwrap();
        cache
            .insert(&bs58_repr, 5, (note, BlsScalar::from(1u64)))
            .unwrap();
        (cache, dir)
    }

    #[tokio::test]
    async fn stale_cache_errors_when_reset_not_allowed() {
        let mut rng = StdRng::seed_from_u64(1);
        let pk = PhoenixPublicKey::from(&PhoenixSecretKey::random(&mut rng));
        let note = gen_note(&mut rng, &pk, 100);
        let (cache, _dir) = cache_with_note(&pk, note);

        let result = check_stale_cache(
            &cache,
            &[pk],
            |_pos| async { Ok::<Option<Note>, Error>(None) },
            false,
            |_| {},
        )
        .await;

        assert!(matches!(result, Err(Error::StaleCache(_))));
    }

    #[tokio::test]
    async fn stale_cache_resets_notes_when_allowed() {
        let mut rng = StdRng::seed_from_u64(2);
        let pk = PhoenixPublicKey::from(&PhoenixSecretKey::random(&mut rng));
        let note = gen_note(&mut rng, &pk, 100);
        let (cache, _dir) = cache_with_note(&pk, note);

        let result = check_stale_cache(
            &cache,
            &[pk],
            |_pos| async { Ok::<Option<Note>, Error>(None) },
            true,
            |_| {},
        )
        .await;

        assert_eq!(result.expect("expected Ok(true)"), true);
        assert!(cache.notes(&pk).unwrap().is_empty());
    }
}
