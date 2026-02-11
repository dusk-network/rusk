// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::transfer::phoenix::{Note, NoteLeaf, ViewKey};
use dusk_core::transfer::TRANSFER_CONTRACT;
use futures_util::Stream;
use rusk::{Error, Result, Rusk};
use tracing::info;

use dusk_bytes::DeserializableSlice;
use std::pin::Pin;
use std::sync::mpsc;
use tokio::spawn;
use tracing::error;

pub type StoredNote = (Note, u64);
pub type GetNotesStream = Pin<Box<dyn Stream<Item = StoredNote> + Send>>;

pub async fn get_notes(
    rusk: &Rusk,
    vk: &[u8],
    height: u64,
) -> Result<GetNotesStream, Error> {
    info!("Received GetNotes request");

    let vk = match vk.is_empty() {
        false => {
            let vk = ViewKey::from_slice(vk).map_err(Error::Serialization)?;
            Some(vk)
        }
        true => None,
    };

    let (sender, receiver) = mpsc::channel();

    // Clone rusk and move it to the thread
    let rusk = rusk.clone();

    // Spawn a task responsible for running the feeder query.
    spawn(async move {
        if let Err(err) = rusk.feeder_query(
            TRANSFER_CONTRACT,
            "leaves_from_height",
            &height,
            sender,
            None,
        ) {
            error!("GetNotes errored: {err}");
        }
    });

    // Make a stream from the receiver and map the elements to be the
    // expected output
    let stream =
        tokio_stream::iter(receiver.into_iter().filter_map(move |bytes| {
            let leaf = rkyv::from_bytes::<NoteLeaf>(&bytes)
                .expect("The contract should always return valid leaves");
            match &vk {
                Some(vk) => vk
                    .owns(leaf.note.stealth_address())
                    .then_some((leaf.note, leaf.block_height)),
                None => Some((leaf.note, leaf.block_height)),
            }
        }));

    Ok(Box::pin(stream) as GetNotesStream)
}
