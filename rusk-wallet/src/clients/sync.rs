// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use futures::StreamExt;
use rues::CONTRACTS_TARGET;

use dusk_bytes::Serializable;
use dusk_core::transfer::phoenix::{
    NoteLeaf, PublicKey as PhoenixPublicKey, SecretKey as PhoenixSecretKey,
    ViewKey as PhoenixViewKey,
};
use dusk_core::BlsScalar;
use wallet_core::keys::{
    derive_phoenix_pk, derive_phoenix_sk, derive_phoenix_vk,
};
use zeroize::Zeroize;

use super::{rues, LocalStore, RuesHttpClient, MAX_PROFILES, TREE_LEAF};

use crate::clients::{Cache, TRANSFER_CONTRACT};
use crate::Error;

pub(crate) async fn sync_db(
    client: &RuesHttpClient,
    cache: &Cache,
    store: &LocalStore,
    status: fn(&str),
) -> Result<(), Error> {
    let seed = store.get_seed();

    let keys: Vec<(PhoenixSecretKey, PhoenixViewKey, PhoenixPublicKey)> = (0
        ..MAX_PROFILES)
        .map(|i| {
            // we know that `i < MAX_PROFILES <= u8::MAX`, so casting to u8 is
            // safe here
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

    let last_pos = cache.last_pos()?;
    let pos_to_search = last_pos.map(|p| p + 1).unwrap_or_default();
    let mut last_pos = last_pos.unwrap_or_default();

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

        buffer = leaf_chunk.remainder().to_vec();
    }

    for (sk, vk, pk) in &keys {
        let pk_bs58 = bs58::encode(pk.to_bytes()).into_string();
        for (block_height, note) in &note_data {
            if vk.owns(note.stealth_address()) {
                let nullifier = note.gen_nullifier(sk);
                let spent =
                    fetch_existing_nullifiers_remote(client, &[nullifier])
                        .await?
                        .first()
                        .is_some();

                let note = (note.clone(), nullifier);

                if spent {
                    cache.insert_spent(&pk_bs58, *block_height, note)?;
                } else {
                    cache.insert(&pk_bs58, *block_height, note)?;
                }
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
                    .await?;

            cache.spend_notes(&pk, existing.as_slice())?;
        }

        sk.zeroize();
    }

    // insert last post after the notes has been inserted
    // to prevent false reporting of sync completion
    cache.insert_last_pos(last_pos)?;

    Ok(())
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

    let nullifiers = rkyv::from_bytes(&data).map_err(|_| Error::Rkyv)?;

    Ok(nullifiers)
}
