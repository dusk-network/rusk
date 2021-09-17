// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_jubjub::JubJubExtended;
use dusk_pki::{PublicSpendKey, ViewKey};
use dusk_plonk::prelude::*;
use phoenix_core::{Error as PhoenixError, Message, Note};

#[derive(Default, Debug, Clone, Copy)]
pub struct CircuitValueOpening {
    value: u64,
    blinder: JubJubScalar,
    commitment: JubJubExtended,
}

impl CircuitValueOpening {
    pub fn from_message(
        message: &Message,
        psk: &PublicSpendKey,
        entropy: &JubJubScalar,
    ) -> Result<Self, PhoenixError> {
        let (value, blinder) = message.decrypt(entropy, psk)?;
        let commitment = *message.value_commitment();

        Ok(Self {
            value,
            blinder,
            commitment,
        })
    }

    pub fn from_note(
        note: &Note,
        vk: Option<&ViewKey>,
    ) -> Result<Self, PhoenixError> {
        let value = note.value(vk)?;
        let blinder = note.blinding_factor(vk)?;
        let commitment = *note.value_commitment();

        Ok(Self {
            value,
            blinder,
            commitment,
        })
    }

    pub const fn value(&self) -> u64 {
        self.value
    }

    pub const fn blinder(&self) -> &JubJubScalar {
        &self.blinder
    }

    pub const fn commitment(&self) -> &JubJubExtended {
        &self.commitment
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CircuitDeriveKey {
    is_public: BlsScalar,
    secret: PublicSpendKey,
    public: PublicSpendKey,
}

impl CircuitDeriveKey {
    pub fn new(psk: PublicSpendKey, public: bool) -> Self {
        let is_public = BlsScalar::from(public as u64);

        let identity = JubJubAffine::identity().into();
        let identity = PublicSpendKey::new(identity, identity);

        let secret = public.then(|| identity).unwrap_or(psk);
        let public = public.then(|| psk).unwrap_or(identity);

        Self {
            is_public,
            secret,
            public,
        }
    }

    pub const fn is_public(&self) -> &BlsScalar {
        &self.is_public
    }

    pub const fn secret(&self) -> &PublicSpendKey {
        &self.secret
    }

    pub fn secret_a(&self) -> &JubJubExtended {
        self.secret.A()
    }

    pub fn secret_b(&self) -> &JubJubExtended {
        self.secret.B()
    }

    pub const fn public(&self) -> &PublicSpendKey {
        &self.secret
    }

    pub fn public_a(&self) -> &JubJubExtended {
        self.public.A()
    }

    pub fn public_b(&self) -> &JubJubExtended {
        self.public.B()
    }
}
