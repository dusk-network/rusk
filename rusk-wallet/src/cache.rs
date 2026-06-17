// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::BTreeSet;
use std::path::Path;

use dusk_bytes::{DeserializableSlice, Serializable};
use dusk_core::BlsScalar;
use dusk_core::transfer::phoenix::{
    Note, NoteLeaf, PublicKey as PhoenixPublicKey,
};
use rkyv::Deserialize;
use rocksdb::{DBWithThreadMode, MultiThreaded, Options, WriteBatch};

use crate::WalletStatus;
use crate::clients::TREE_LEAF;
use crate::error::Error;

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
        status: fn(WalletStatus),
    ) -> Result<Self, Error> {
        status(WalletStatus::Info("Opening notes database".into()));

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
                .ok_or(Error::CacheDatabaseCorrupted)?;
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
        let last_pos = self.db.get(b"last_pos")?;

        match last_pos {
            Some(x) => {
                let buff =
                    x.try_into().map_err(|_| Error::CacheDatabaseCorrupted)?;

                Ok(Some(u64::from_be_bytes(buff)))
            }
            None => Ok(None),
        }
    }

    /// Clears all cached notes and resets the sync position.
    pub(crate) fn clear_notes(
        &self,
        pks: &[PhoenixPublicKey],
    ) -> Result<(), Error> {
        let mut batch = WriteBatch::default();
        batch.delete(b"last_pos");

        for pk in pks {
            let bs58 = bs58::encode(pk.to_bytes()).into_string();
            let spent = format!("spent_{bs58}");
            for cf_name in [bs58.as_str(), spent.as_str()] {
                if let Some(cf) = self.db.cf_handle(cf_name) {
                    for item in
                        self.db.iterator_cf(&cf, rocksdb::IteratorMode::Start)
                    {
                        let (key, _) = item?;
                        batch.delete_cf(&cf, key);
                    }
                }
            }
        }

        self.db.write(batch)?;
        Ok(())
    }

    /// Returns an arbitrary owned note with `block_height > 0`,
    /// used to verify the cache against the live chain.
    pub(crate) fn note_for_cache_validation(
        &self,
        pks: &[PhoenixPublicKey],
    ) -> Result<Option<Note>, Error> {
        for pk in pks {
            let bs58 = bs58::encode(pk.to_bytes()).into_string();
            let spent = format!("spent_{bs58}");
            for cf_name in [bs58.as_str(), spent.as_str()] {
                let Some(cf) = self.db.cf_handle(cf_name) else {
                    continue;
                };
                for item in
                    self.db.iterator_cf(&cf, rocksdb::IteratorMode::Start)
                {
                    let (_, data) = item?;
                    let leaf: NoteLeaf =
                        rkyv::check_archived_root::<NoteLeaf>(&data)
                            .map_err(|_| Error::CacheDatabaseCorrupted)?
                            .deserialize(&mut rkyv::Infallible)
                            .unwrap();
                    if leaf.block_height > 0 {
                        return Ok(Some(leaf.note));
                    }
                }
            }
        }
        Ok(None)
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
        }

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

                let note = rkyv::check_archived_root::<NoteLeaf>(&note_data)
                    .map_err(|_| Error::CacheDatabaseCorrupted)?
                    .deserialize(&mut rkyv::Infallible)
                    .unwrap();

                notes.insert(note);
            }
        }

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

                let note = rkyv::check_archived_root::<NoteLeaf>(&note_data)
                    .map_err(|_| Error::CacheDatabaseCorrupted)?
                    .deserialize(&mut rkyv::Infallible)
                    .unwrap();

                let key = BlsScalar::from_slice(&key)?;

                notes.push((key, note));
            }
        }

        Ok(notes)
    }

    pub fn close(&self) {
        self.db.cancel_all_background_work(false);
    }
}

#[cfg(test)]
mod tests {
    use dusk_core::JubJubScalar;
    use dusk_core::transfer::phoenix::SecretKey as PhoenixSecretKey;
    use ff::Field;
    use rand::rngs::StdRng;
    use rand::{CryptoRng, RngCore, SeedableRng};
    use tempfile::tempdir;

    use super::*;

    fn gen_note<T: RngCore + CryptoRng>(
        rng: &mut T,
        owner_pk: &PhoenixPublicKey,
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
            100,
            value_blinder,
            sender_blinder,
        )
    }

    fn make_cache(pks: &[PhoenixPublicKey]) -> (Cache, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let cfs = pks
            .iter()
            .flat_map(|pk| {
                let bs58 = bs58::encode(pk.to_bytes()).into_string();
                [bs58.clone(), format!("spent_{bs58}")]
            })
            .collect();
        (Cache::new(dir.path(), cfs, |_| {}).unwrap(), dir)
    }

    #[test]
    fn clear_notes_wipes_notes_and_last_pos() {
        let mut rng = StdRng::seed_from_u64(42);
        let pk = PhoenixPublicKey::from(&PhoenixSecretKey::random(&mut rng));
        let pk_bs58 = bs58::encode(pk.to_bytes()).into_string();
        let (cache, _dir) = make_cache(&[pk]);

        cache
            .insert(
                &pk_bs58,
                1,
                (gen_note(&mut rng, &pk), BlsScalar::from(1u64)),
            )
            .unwrap();
        cache.insert_last_pos(0).unwrap();

        cache.clear_notes(&[pk]).unwrap();

        assert!(cache.notes(&pk).unwrap().is_empty());
        assert!(cache.last_pos().unwrap().is_none());
    }

    #[test]
    fn note_for_cache_validation_skips_genesis() {
        let mut rng = StdRng::seed_from_u64(42);
        let pk = PhoenixPublicKey::from(&PhoenixSecretKey::random(&mut rng));
        let pk_bs58 = bs58::encode(pk.to_bytes()).into_string();
        let (cache, _dir) = make_cache(&[pk]);

        // A genesis note (block_height == 0) must not be returned.
        cache
            .insert(
                &pk_bs58,
                0,
                (gen_note(&mut rng, &pk), BlsScalar::from(1u64)),
            )
            .unwrap();

        assert!(cache.note_for_cache_validation(&[pk]).unwrap().is_none());
    }

    #[test]
    fn note_for_cache_validation_returns_non_genesis() {
        let mut rng = StdRng::seed_from_u64(42);
        let pk = PhoenixPublicKey::from(&PhoenixSecretKey::random(&mut rng));
        let pk_bs58 = bs58::encode(pk.to_bytes()).into_string();
        let (cache, _dir) = make_cache(&[pk]);

        let note = gen_note(&mut rng, &pk);
        cache
            .insert(&pk_bs58, 5, (note.clone(), BlsScalar::from(1u64)))
            .unwrap();

        let found = cache.note_for_cache_validation(&[pk]).unwrap();
        assert_eq!(found, Some(note));
    }
}
