// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Functions that are only available for the host to call.

use crate::{Error, TransferContract};

use dusk_pki::PublicSpendKey;
use phoenix_core::Note;
use rand::{CryptoRng, RngCore};

impl TransferContract {
    /// Adds two notes to the state - one as a reward for the block generator
    /// and another for Dusk foundation. The first note returned is the Dusk
    /// note, and the second the generator note.
    ///
    /// 90% of the value goes to the generator (rounded up).
    /// 10% of the value goes to the Dusk address (rounded down).
    pub fn mint<Rng: RngCore + CryptoRng>(
        &mut self,
        rng: &mut Rng,
        block_height: u64,
        value: u64,
        dusk: &PublicSpendKey,
        generator: &PublicSpendKey,
    ) -> Result<(Note, Note), Error> {
        let dusk_value = value / 10;
        let generator_value = value - dusk_value;

        let dusk_note = Note::transparent(rng, dusk, dusk_value);
        let generator_note = Note::transparent(rng, generator, generator_value);

        let dusk_note = self.push_note(block_height, dusk_note)?;
        let generator_note = self.push_note(block_height, generator_note)?;

        self.update_root()?;

        Ok((dusk_note, generator_note))
    }
}
