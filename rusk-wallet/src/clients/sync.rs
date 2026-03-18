// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::Serializable;
use dusk_core::BlsScalar;
use dusk_core::transfer::phoenix::{
    Note, NoteLeaf, PublicKey as PhoenixPublicKey,
    SecretKey as PhoenixSecretKey, ViewKey as PhoenixViewKey,
};
use futures::StreamExt;
use rkyv::Deserialize;
use wallet_core::keys::{
    derive_phoenix_pk, derive_phoenix_sk, derive_phoenix_vk,
};
use zeroize::Zeroize;

use super::{LocalStore, MAX_PROFILES, TREE_LEAF};
use crate::Error;
use crate::clients::{Cache, TRANSFER_CONTRACT};
use crate::rues::{CONTRACTS_TARGET, HttpClient as RuesHttpClient};

const SYNC_PROGRESS_BLOCK_STEP: u64 = 100;

pub(crate) async fn sync_db(
    client: &RuesHttpClient,
    cache: &Cache,
    store: &LocalStore,
    status: fn(&str),
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
    let pos_to_search = last_pos.map(|p| p + 1).unwrap_or_default();

    if pos_to_search > 0 {
        status(&format!(
            "Resuming sync from cached note position {pos_to_search}"
        ));
    }

    let (last_pos, max_block_height, note_data) =
        collect_fresh_notes(client, pos_to_search, &mut keys, status).await?;

    let mut err = Ok(());
    'outer: for (sk, vk, pk) in &keys {
        let pk_bs58 = bs58::encode(pk.to_bytes()).into_string();
        for (block_height, note) in &note_data {
            if vk.owns(note.stealth_address()) {
                let nullifier = note.gen_nullifier(sk);
                let result =
                    fetch_existing_nullifiers_remote(client, &[nullifier])
                        .await
                        .and_then(|fetch_res| {
                            let spent = !fetch_res.is_empty();
                            let note = (note.clone(), nullifier);
                            if spent {
                                cache.insert_spent(
                                    &pk_bs58,
                                    *block_height,
                                    note,
                                )?;
                            } else {
                                cache.insert(&pk_bs58, *block_height, note)?;
                            }
                            Ok(())
                        });
                if result.is_err() {
                    err = result;
                    break 'outer;
                }
            }
        }
    }

    zeroize_secret_keys(&mut keys);
    err?;

    // Remove spent nullifiers from live notes
    // zerorize all the secret keys
    for (_, _, pk) in keys {
        let nullifiers: Vec<BlsScalar> = cache.unspent_notes_id(&pk)?;

        if !nullifiers.is_empty() {
            let existing =
                fetch_existing_nullifiers_remote(client, nullifiers.as_slice())
                    .await?;

            cache.spend_notes(&pk, existing.as_slice())?;
        }
    }

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
        .call_raw(
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

fn zeroize_secret_keys(
    keys: &mut [(PhoenixSecretKey, PhoenixViewKey, PhoenixPublicKey)],
) {
    for (sk, _, _) in keys.iter_mut() {
        sk.zeroize();
    }
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
        .contract_query::<_, _, 1024>(
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
