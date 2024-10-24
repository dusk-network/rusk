// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::BTreeSet;
use std::path::Path;

use dusk_bytes::{DeserializableSlice, Serializable};
use execution_core::transfer::phoenix::NoteLeaf;
use rocksdb::{DBWithThreadMode, MultiThreaded, Options};

use super::*;

use crate::{clients::TREE_LEAF, error::Error};

type DB = DBWithThreadMode<MultiThreaded>;

/// A cache of notes received from Rusk.
///
/// path is the path of the rocks db database
pub(crate) struct Cache {
    db: DB,
}

impl Cache {
    /// Returns a new cache instance.
    pub(crate) fn new<T: AsRef<Path>>(
        path: T,
        cfs: Vec<String>,
        status: fn(&str),
    ) -> Result<Self, Error> {
        status("Opening notes database");

        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        // After 10 million bytes, sort the cache file and create new one
        opts.set_write_buffer_size(10_000_000);

        // create all CF(s) on startup if we don't have them
        let db = DB::open_cf(&opts, path, cfs)?;

        Ok(Self { db })
    }

    // We store a column family named by hex representation of the pk.
    // We store the nullifier of the note as key and the value is the bytes
    // representation of the tuple (NoteHeight, Note)
    pub(crate) fn insert(
        &self,
        pk_bs58: &str,
        block_height: u64,
        note_data: (Note, BlsScalar),
    ) -> Result<(), Error> {
        let cf_name = pk_bs58;

        let cf = self
            .db
            .cf_handle(cf_name)
            .ok_or(Error::CacheDatabaseCorrupted)?;

        let (note, nullifier) = note_data;

        let leaf = NoteLeaf { block_height, note };

        let data = rkyv::to_bytes::<NoteLeaf, TREE_LEAF>(&leaf)
            .map_err(|_| Error::Rkyv)?;

        let key = nullifier.to_bytes();

        self.db.put_cf(&cf, key, data)?;

        Ok(())
    }

    // We store a column family named by hex representation of the pk.
    // We store the nullifier of the note as key and the value is the bytes
    // representation of the tuple (NoteHeight, Note)
    pub(crate) fn insert_spent(
        &self,
        pk_bs58: &str,
        block_height: u64,
        note_data: (Note, BlsScalar),
    ) -> Result<(), Error> {
        let cf_name = format!("spent_{pk_bs58}");

        let cf = self
            .db
            .cf_handle(&cf_name)
            .ok_or(Error::CacheDatabaseCorrupted)?;

        let (note, nullifier) = note_data;

        let leaf = NoteLeaf { block_height, note };
        let data = rkyv::to_bytes::<NoteLeaf, TREE_LEAF>(&leaf)
            .map_err(|_| Error::Rkyv)?;
        let key = nullifier.to_bytes();

        self.db.put_cf(&cf, key, data)?;

        Ok(())
    }

    pub(crate) fn spend_notes(
        &self,
        pk: &PhoenixPublicKey,
        nullifiers: &[BlsScalar],
    ) -> Result<(), Error> {
        if nullifiers.is_empty() {
            return Ok(());
        }

        let pk_bs58 = bs58::encode(pk.to_bytes()).into_string();

        let spent_cf_name = format!("spent_{pk_bs58}");
        let cf_name = pk_bs58;

        let cf = self
            .db
            .cf_handle(&cf_name)
            .ok_or(Error::CacheDatabaseCorrupted)?;
        let spent_cf = self
            .db
            .cf_handle(&spent_cf_name)
            .ok_or(Error::CacheDatabaseCorrupted)?;

        for n in nullifiers {
            let key = n.to_bytes();
            let to_move = self
                .db
                .get_cf(&cf, key)?
                .expect("Note must exists to be moved");
            self.db.put_cf(&spent_cf, key, to_move)?;
            self.db.delete_cf(&cf, n.to_bytes())?;
        }

        Ok(())
    }

    pub(crate) fn insert_last_pos(&self, last_pos: u64) -> Result<(), Error> {
        self.db.put(b"last_pos", last_pos.to_be_bytes())?;

        Ok(())
    }

    /// Returns the last position of inserted notes. If no note has ever been
    /// inserted it returns None.
    pub(crate) fn last_pos(&self) -> Result<Option<u64>, Error> {
        Ok(self.db.get(b"last_pos")?.map(|x| {
            let buff: [u8; 8] = x.try_into().expect("Invalid u64 in cache db");

            u64::from_be_bytes(buff)
        }))
    }

    /// Returns an iterator over all unspent notes nullifier for the given pk.
    pub(crate) fn unspent_notes_id(
        &self,
        pk: &PhoenixPublicKey,
    ) -> Result<Vec<BlsScalar>, Error> {
        let cf_name = bs58::encode(pk.to_bytes()).into_string();
        let mut notes = vec![];

        if let Some(cf) = self.db.cf_handle(&cf_name) {
            let iterator =
                self.db.iterator_cf(&cf, rocksdb::IteratorMode::Start);

            for i in iterator {
                let (id, _) = i?;

                let id = BlsScalar::from_slice(&id)?;
                notes.push(id);
            }
        };

        Ok(notes)
    }

    /// Returns an iterator over all unspent notes inserted for the given pk,
    /// in order of note position.
    pub(crate) fn notes(
        &self,
        pk: &PhoenixPublicKey,
    ) -> Result<BTreeSet<NoteLeaf>, Error> {
        let cf_name = bs58::encode(pk.to_bytes()).into_string();
        let mut notes = BTreeSet::<NoteLeaf>::new();

        if let Some(cf) = self.db.cf_handle(&cf_name) {
            let iterator =
                self.db.iterator_cf(&cf, rocksdb::IteratorMode::Start);

            for i in iterator {
                let (_, note_data) = i?;

                let note = rkyv::from_bytes(&note_data)
                    .map_err(|_| Error::CacheDatabaseCorrupted)?;

                notes.insert(note);
            }
        };

        Ok(notes)
    }

    /// Returns an iterator over all notes inserted for the given pk, in order
    /// of block height.
    pub(crate) fn spent_notes(
        &self,
        pk: &PhoenixPublicKey,
    ) -> Result<Vec<(BlsScalar, NoteLeaf)>, Error> {
        let pk_bs58 = bs58::encode(pk.to_bytes()).into_string();
        let cf_name = format!("spent_{pk_bs58}");
        let mut notes = vec![];

        if let Some(cf) = self.db.cf_handle(&cf_name) {
            let iterator =
                self.db.iterator_cf(&cf, rocksdb::IteratorMode::Start);

            for i in iterator {
                let (key, note_data) = i?;

                let note = rkyv::from_bytes(&note_data)
                    .map_err(|_| Error::CacheDatabaseCorrupted)?;

                let key = BlsScalar::from_slice(&key)?;

                notes.push((key, note));
            }
        };

        Ok(notes)
    }

    pub fn close(&self) {
        self.db.cancel_all_background_work(false);
    }
}
