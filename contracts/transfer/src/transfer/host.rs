// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Functions that are only available for the host to call.

use crate::{Error, TransferContract};

use dusk_bytes::Serializable;
use dusk_pki::PublicSpendKey;
use lazy_static::lazy_static;
use phoenix_core::Note;
use rand::rngs::OsRng;

lazy_static! {
    /// The key Dusk is paid to.
    pub static ref DUSK_KEY: PublicSpendKey =  {
        let key_bytes = include_bytes!("../../dusk.psk");
        PublicSpendKey::from_bytes(key_bytes).expect("Dusk's key must be valid")
    };
}

impl TransferContract {
    /// Adds two notes to the state - one as a reward for the block generator
    /// and another for Dusk foundation. The first note returned is the Dusk
    /// note, and the second the generator note.
    pub fn mint(
        &mut self,
        block_height: u64,
        dusk_value: u64,
        generator_value: u64,
        generator: Option<&PublicSpendKey>,
    ) -> Result<(Note, Note), Error> {
        let mut rng = OsRng::default();

        let generator = generator.unwrap_or(&DUSK_KEY);

        let dusk_note = Note::transparent(&mut rng, &DUSK_KEY, dusk_value);
        let generator_note =
            Note::transparent(&mut rng, generator, generator_value);

        let dusk_note = self.push_note(block_height, dusk_note)?;
        let generator_note = self.push_note(block_height, generator_note)?;

        self.update_root()?;

        Ok((dusk_note, generator_note))
    }
}
