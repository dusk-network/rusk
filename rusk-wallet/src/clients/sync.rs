// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::mem::size_of;

use futures::StreamExt;

use crate::block::Block;
use crate::clients::{Cache, TRANSFER_CONTRACT};
use crate::rusk::RuskHttpClient;
use crate::{Error, RuskRequest};

use super::*;

const TREE_LEAF: usize = size_of::<ArchivedNoteLeaf>();

pub(crate) async fn sync_db(
    client: &RuskHttpClient,
    cache: &Cache,
    store: &LocalStore,
    status: fn(&str),
) -> Result<(), Error> {
    let seed = store.get_seed();

    let keys: Vec<(PhoenixSecretKey, PhoenixViewKey, PhoenixPublicKey)> = (0
        ..MAX_ADDRESSES)
        .map(|i| {
            let i = i as u8;
            (
                derive_phoenix_sk(seed, i),
                derive_phoenix_vk(seed, i),
                derive_phoenix_pk(seed, i),
            )
        })
        .collect();

    status("Getting cached note position...");

    let last_pos = cache.last_pos()?;
    let pos_to_search = last_pos.map(|p| p + 1).unwrap_or_default();
    let mut last_pos = last_pos.unwrap_or_default();

    status("Fetching fresh notes...");

    let req = rkyv::to_bytes::<_, 8>(&(pos_to_search))
        .map_err(|_| Error::Rkyv)?
        .to_vec();

    let mut stream = client
        .call_raw(
            1,
            TRANSFER_CONTRACT,
            &RuskRequest::new("leaves_from_pos", req),
            true,
        )
        .await?
        .bytes_stream();

    status("Connection established...");

    status("Streaming notes...");

    // This buffer is needed because `.bytes_stream();` introduce additional
    // spliting of chunks according to it's own buffer
    let mut buffer = vec![];
    let mut note_data = Vec::new();

    while let Some(http_chunk) = stream.next().await {
        buffer.extend_from_slice(&http_chunk?);

        let mut leaf_chunk = buffer.chunks_exact(TREE_LEAF);

        for leaf_bytes in leaf_chunk.by_ref() {
            let NoteLeaf { block_height, note } =
                rkyv::from_bytes(leaf_bytes).map_err(|_| Error::Rkyv)?;

            last_pos = std::cmp::max(last_pos, *note.pos());

            note_data.push((block_height, note));
        }

        cache.insert_last_pos(last_pos)?;

        buffer = leaf_chunk.remainder().to_vec();
    }

    for (sk, vk, pk) in keys.iter() {
        for (block_height, note) in note_data.iter() {
            if vk.owns(note.stealth_address()) {
                let nullifier = note.gen_nullifier(sk);
                let spent =
                    fetch_existing_nullifiers_remote(client, &[nullifier])
                        .wait()?
                        .first()
                        .is_some();
                let note = (note.clone(), nullifier);

                match spent {
                    true => cache.insert_spent(pk, *block_height, note),
                    false => cache.insert(pk, *block_height, note),
                }?;
            }
        }
    }

    // Remove spent nullifiers from live notes
    // zerorize all the secret keys
    for (mut sk, _, pk) in keys {
        let nullifiers: Vec<BlsScalar> = cache.unspent_notes_id(&pk)?;

        if !nullifiers.is_empty() {
            let existing =
                fetch_existing_nullifiers_remote(client, nullifiers.as_slice())
                    .wait()?;

            cache.spend_notes(&pk, existing.as_slice())?;
        }

        sk.zeroize();
    }

    Ok(())
}

/// Asks the node to return the nullifiers that already exist from the given
/// nullifiers.
pub(crate) async fn fetch_existing_nullifiers_remote(
    client: &RuskHttpClient,
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

    let nullifiers = rkyv::from_bytes(&data).map_err(|_| Error::Rkyv)?;

    Ok(nullifiers)
}
