// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;
use crate::error::Error;
use crate::node::{Rusk, RuskTip};

use std::pin::Pin;
use std::sync::mpsc;

use dusk_bytes::DeserializableSlice;
use futures::Stream;
use tokio::spawn;
use tracing::{error, info};

use dusk_core::{
    signatures::bls::PublicKey as BlsPublicKey,
    stake::{StakeData, STAKE_CONTRACT},
    transfer::{
        phoenix::{Note, NoteLeaf, NoteOpening, ViewKey},
        TRANSFER_CONTRACT,
    },
    BlsScalar,
};
use dusk_vm::VM;
use parking_lot::RwLockWriteGuard;

pub type StoredNote = (Note, u64);

pub type GetNotesStream = Pin<Box<dyn Stream<Item = StoredNote> + Send>>;

impl Rusk {
    /// Performs a feeder query returning the leaves of the transfer tree
    /// starting from the given height. The function will block while executing,
    /// and the results of the query will be passed through the `receiver`
    /// counterpart of the given `sender`.
    ///
    /// The receiver of the leaves is responsible for deserializing the leaves
    /// appropriately - i.e. using `rkyv`.
    pub fn leaves_from_height(
        &self,
        height: u64,
        sender: mpsc::Sender<Vec<u8>>,
    ) -> Result<()> {
        self.feeder_query(
            TRANSFER_CONTRACT,
            "leaves_from_height",
            &height,
            sender,
            None,
        )
    }

    /// Returns the root of the transfer tree.
    pub fn tree_root(&self) -> Result<BlsScalar> {
        info!("Received tree_root request");
        self.query_root()
    }

    /// Returns the opening of the transfer tree at the given position.
    pub fn tree_opening(&self, pos: u64) -> Result<Option<NoteOpening>> {
        self.query_opening(pos)
    }

    /// Returns data about the stake of the given key.
    pub fn stake(&self, pk: BlsPublicKey) -> Result<Option<StakeData>> {
        self.query(STAKE_CONTRACT, "get_stake", &pk)
    }

    pub async fn get_notes(
        &self,
        vk: &[u8],
        height: u64,
    ) -> Result<GetNotesStream, Error> {
        info!("Received GetNotes request");

        let vk = match vk.is_empty() {
            false => {
                let vk =
                    ViewKey::from_slice(vk).map_err(Error::Serialization)?;
                Some(vk)
            }
            true => None,
        };

        let (sender, receiver) = mpsc::channel();

        // Clone rusk and move it to the thread
        let rusk = self.clone();

        // Spawn a task responsible for running the feeder query.
        spawn(async move {
            if let Err(err) = rusk.leaves_from_height(height, sender) {
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

    /// Perform an action with the underlying data structure.
    ///
    /// This should **not be used** internally, to avoid locking the structure
    /// for too long of a period of time.
    pub fn with_tip<'a, F, T>(&'a self, closure: F) -> T
    where
        F: FnOnce(RwLockWriteGuard<'a, RuskTip>, &'a VM) -> T,
    {
        let tip = self.tip.write();
        closure(tip, &self.vm)
    }
}
