// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::path::Path;

use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_pki::PublicSpendKey;
use dusk_plonk::prelude::BlsScalar;
use dusk_wallet_core::Store;
use phoenix_core::Note;
use rocksdb::{DBWithThreadMode, MultiThreaded, Options};

use crate::{error::Error, store::LocalStore, MAX_ADDRESSES};

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
        store: &LocalStore,
        status: fn(&str),
    ) -> Result<Self, Error> {
        let cfs: Vec<_> = (0..MAX_ADDRESSES)
            .flat_map(|i| {
                let ssk =
                    store.retrieve_ssk(i as u64).expect("ssk to be available");
                let psk = ssk.public_spend_key();

                let live = format!("{:?}", psk);
                let spent = format!("spent_{:?}", psk);
                [live, spent]
            })
            .collect();

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

    // We store a column family named by hex representation of the psk.
    // We store the nullifier of the note as key and the value is the bytes
    // representation of the tuple (NoteHeight, Note)
    pub(crate) fn insert(
        &self,
        psk: &PublicSpendKey,
        height: u64,
        note_data: (Note, BlsScalar),
    ) -> Result<(), Error> {
        let cf_name = format!("{:?}", psk);

        let cf = self
            .db
            .cf_handle(&cf_name)
            .ok_or(Error::CacheDatabaseCorrupted)?;

        let (note, nullifier) = note_data;

        let data = NoteData { height, note };
        let key = nullifier.to_bytes();

        self.db.put_cf(&cf, key, data.to_bytes())?;

        Ok(())
    }

    // We store a column family named by hex representation of the psk.
    // We store the nullifier of the note as key and the value is the bytes
    // representation of the tuple (NoteHeight, Note)
    pub(crate) fn insert_spent(
        &self,
        psk: &PublicSpendKey,
        height: u64,
        note_data: (Note, BlsScalar),
    ) -> Result<(), Error> {
        let cf_name = format!("spent_{:?}", psk);

        let cf = self
            .db
            .cf_handle(&cf_name)
            .ok_or(Error::CacheDatabaseCorrupted)?;

        let (note, nullifier) = note_data;

        let data = NoteData { height, note };
        let key = nullifier.to_bytes();

        self.db.put_cf(&cf, key, data.to_bytes())?;

        Ok(())
    }

    pub(crate) fn spend_notes(
        &self,
        psk: &PublicSpendKey,
        nullifiers: &[BlsScalar],
    ) -> Result<(), Error> {
        if nullifiers.is_empty() {
            return Ok(());
        }
        let cf_name = format!("{:?}", psk);
        let spent_cf_name = format!("spent_{:?}", psk);

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

    /// Returns an iterator over all unspent notes nullifier for the given PSK.
    pub(crate) fn unspent_notes_id(
        &self,
        psk: &PublicSpendKey,
    ) -> Result<Vec<BlsScalar>, Error> {
        let cf_name = format!("{:?}", psk);
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

    /// Returns an iterator over all unspent notes inserted for the given PSK,
    /// in order of note position.
    pub(crate) fn notes(
        &self,
        psk: &PublicSpendKey,
    ) -> Result<BTreeSet<NoteData>, Error> {
        let cf_name = format!("{:?}", psk);
        let mut notes = BTreeSet::<NoteData>::new();

        if let Some(cf) = self.db.cf_handle(&cf_name) {
            let iterator =
                self.db.iterator_cf(&cf, rocksdb::IteratorMode::Start);

            for i in iterator {
                let (_, note_data) = i?;

                let note = NoteData::from_slice(&note_data)?;

                notes.insert(note);
            }
        };

        Ok(notes)
    }

    /// Returns an iterator over all notes inserted for the given PSK, in order
    /// of block height.
    pub(crate) fn spent_notes(
        &self,
        psk: &PublicSpendKey,
    ) -> Result<Vec<(BlsScalar, NoteData)>, Error> {
        let cf_name = format!("spent_{:?}", psk);
        let mut notes = vec![];

        if let Some(cf) = self.db.cf_handle(&cf_name) {
            let iterator =
                self.db.iterator_cf(&cf, rocksdb::IteratorMode::Start);

            for i in iterator {
                let (key, note_data) = i?;

                let note = NoteData::from_slice(&note_data)?;
                let key = BlsScalar::from_slice(&key)?;

                notes.push((key, note));
            }
        };

        Ok(notes)
    }
}

/// Data kept about each note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NoteData {
    pub height: u64,
    pub note: Note,
}

impl PartialOrd for NoteData {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NoteData {
    fn cmp(&self, other: &Self) -> Ordering {
        self.note.pos().cmp(other.note.pos())
    }
}

impl Serializable<{ u64::SIZE + Note::SIZE }> for NoteData {
    type Error = dusk_bytes::Error;
    /// Converts a Note into a byte representation

    fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];

        buf[0..8].copy_from_slice(&self.height.to_bytes());

        buf[8..].copy_from_slice(&self.note.to_bytes());

        buf
    }

    /// Attempts to convert a byte representation of a note into a `Note`,
    /// failing if the input is invalid
    fn from_bytes(bytes: &[u8; Self::SIZE]) -> Result<Self, Self::Error> {
        let mut one_u64 = [0u8; 8];
        one_u64.copy_from_slice(&bytes[0..8]);
        let height = u64::from_bytes(&one_u64)?;

        let note = Note::from_slice(&bytes[8..])?;
        Ok(Self { height, note })
    }
}
