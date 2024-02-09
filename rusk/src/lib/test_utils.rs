// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;
use crate::chain::Rusk;
use crate::error::Error;

use std::pin::Pin;
use std::sync::mpsc;

use dusk_bytes::DeserializableSlice;
use futures::Stream;
use tokio::spawn;
use tracing::{error, info};

use dusk_bls12_381::BlsScalar;
use dusk_bls12_381_sign::PublicKey as BlsPublicKey;
use dusk_pki::{PublicKey, ViewKey};
use phoenix_core::transaction::{StakeData, TreeLeaf, TRANSFER_TREE_DEPTH};
use phoenix_core::{Message, Note};
use poseidon_merkle::Opening as PoseidonOpening;
use rusk_abi::{ContractId, STAKE_CONTRACT, TRANSFER_CONTRACT};

const A: usize = 4;

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
        self.query(TRANSFER_CONTRACT, "root", &())
    }

    /// Returns the opening of the transfer tree at the given position.
    pub fn tree_opening(
        &self,
        pos: u64,
    ) -> Result<Option<PoseidonOpening<(), TRANSFER_TREE_DEPTH, A>>> {
        self.query(TRANSFER_CONTRACT, "opening", &pos)
    }

    /// Returns the "transparent" balance of the given module.
    pub fn module_balance(&self, contract: ContractId) -> Result<u64> {
        self.query(TRANSFER_CONTRACT, "module_balance", &contract)
    }

    /// Returns the message mapped to the given module and public key.
    pub fn module_message(
        &self,
        contract: ContractId,
        pk: PublicKey,
    ) -> Result<Option<Message>> {
        self.query(TRANSFER_CONTRACT, "message", &(contract, pk))
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
                let leaf = rkyv::from_bytes::<TreeLeaf>(&bytes)
                    .expect("The contract should always return valid leaves");
                match &vk {
                    Some(vk) => vk
                        .owns(&leaf.note)
                        .then_some((leaf.note, leaf.block_height)),
                    None => Some((leaf.note, leaf.block_height)),
                }
            }));

        Ok(Box::pin(stream) as GetNotesStream)
    }
}
