// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::HashMap;
use std::path::Path;

use dusk_bytes::{DeserializableSlice, Serializable};
use phoenix_core::Note;
use rusqlite::{params, Connection, Error};

const TABLE_NOTES: &str =
    "CREATE TABLE if not exists notes (note BLOB, spendkey BLOB)";
const TABLE_CACHE: &str =
    "CREATE TABLE if not exists cache (block BIGINT, spendkey BLOB)";

const QUERY_BLOCK_HEIGHT: &str = "SELECT block FROM cache WHERE spendkey = ?";
const UPDATE_BLOCK_HEIGHT: &str = "update cache set block=? WHERE spendkey = ?";
const INSERT_BLOCK_HEIGHT: &str =
    "INSERT INTO cache (block, spendkey) VALUES (?1, ?2)";

const INSERT_NOTES: &str = "INSERT INTO notes (note, spendkey) values (?1, ?2)";
const QUERY_NOTES: &str = "SELECT note FROM notes WHERE spendkey = ?";

pub struct Cache(Connection);

impl Cache {
    pub(crate) fn new(data_dir: &Path) -> Result<Self, Error> {
        let db_path = data_dir.join("cache.db");

        let cache = Connection::open(db_path)?;
        cache.execute(TABLE_NOTES, [])?;
        cache.execute(TABLE_CACHE, [])?;
        Ok(Cache(cache))
    }

    pub(crate) fn last_block_height(&self, spendkey: &[u8]) -> u64 {
        let cached_block_height =
            self.0
                .query_row_and_then(QUERY_BLOCK_HEIGHT, [spendkey], |row| {
                    row.get(0)
                });
        cached_block_height.unwrap_or(0u64)
    }

    pub(crate) fn persist_block_height(
        &self,
        spendkey: &[u8],
        current_block: u64,
    ) -> Result<(), Error> {
        if self
            .0
            .execute(UPDATE_BLOCK_HEIGHT, params!(current_block, spendkey))?
            == 0
        {
            self.0.execute(
                INSERT_BLOCK_HEIGHT,
                params!(current_block, spendkey),
            )?;
        }
        Ok(())
    }

    pub(crate) fn persist_notes(
        &self,
        spendkey: &[u8],
        notes: &[Note],
    ) -> Result<(), Error> {
        for n in notes.iter() {
            let mut insert_stats = self.0.prepare(INSERT_NOTES)?;
            insert_stats.execute([n.to_bytes().to_vec(), spendkey.to_vec()])?;
        }
        Ok(())
    }

    pub(crate) fn cached_notes(
        &self,
        spendkey: &[u8],
    ) -> Result<HashMap<Vec<u8>, Note>, Error> {
        let mut notes: HashMap<Vec<u8>, Note> = HashMap::new();

        let mut stmt = self.0.prepare(QUERY_NOTES)?;
        let mut rows = stmt.query([spendkey])?;
        while let Some(row) = rows.next()? {
            let note_bytes: Vec<u8> = row.get(0)?;
            let note = Note::from_slice(&note_bytes[..])
                .expect("Invalid notes previously saved");
            notes.insert(note.hash().to_bytes().to_vec(), note);
        }
        Ok(notes)
    }
}
