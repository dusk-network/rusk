// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_pki::PublicSpendKey;

use dusk_plonk::prelude::*;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct DeriveKey {
    pub(crate) is_public: bool,
    pub(crate) secret_a: JubJubExtended,
    pub(crate) secret_b: JubJubExtended,
    pub(crate) public_a: JubJubExtended,
    pub(crate) public_b: JubJubExtended,
}

impl DeriveKey {
    pub fn new(is_public: bool, psk: &PublicSpendKey) -> Self {
        let i = JubJubExtended::identity();

        let a = *psk.A();
        let b = *psk.B();

        let (secret_a, secret_b) = if is_public { (i, i) } else { (a, b) };
        let (public_a, public_b) = if is_public { (a, b) } else { (i, i) };

        Self {
            is_public,
            secret_a,
            secret_b,
            public_a,
            public_b,
        }
    }

    pub const fn is_public(&self) -> bool {
        self.is_public
    }

    pub const fn secret_a(&self) -> &JubJubExtended {
        &self.secret_a
    }

    pub const fn secret_b(&self) -> &JubJubExtended {
        &self.secret_b
    }

    pub const fn public_a(&self) -> &JubJubExtended {
        &self.public_a
    }

    pub const fn public_b(&self) -> &JubJubExtended {
        &self.public_b
    }

    pub const fn into_inner(
        self,
    ) -> (
        bool,
        JubJubExtended,
        JubJubExtended,
        JubJubExtended,
        JubJubExtended,
    ) {
        (
            self.is_public,
            self.secret_a,
            self.secret_b,
            self.public_a,
            self.public_b,
        )
    }
}
