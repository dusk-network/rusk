// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use canonical::Canon;
use canonical_derive::Canon;
use dusk_bls12_381::BlsScalar;
use dusk_jubjub::{JubJubAffine, JubJubScalar};

/// Enum that represents all possible types of public inputs
#[derive(Canon, Clone)]
pub enum PublicInput {
    /// A Public Input Point
    Point(JubJubAffine),
    /// A Public Input BLS Scalar
    BlsScalar(BlsScalar),
    /// A Public Input JubJub Scalar
    JubJubScalar(JubJubScalar),
}

impl From<BlsScalar> for PublicInput {
    fn from(s: BlsScalar) -> PublicInput {
        Self::BlsScalar(s)
    }
}

impl From<JubJubScalar> for PublicInput {
    fn from(s: JubJubScalar) -> PublicInput {
        Self::JubJubScalar(s)
    }
}

impl From<JubJubAffine> for PublicInput {
    fn from(p: JubJubAffine) -> PublicInput {
        Self::Point(p)
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod host {
    use super::PublicInput;
    use dusk_plonk::prelude::*;

    impl From<PublicInput> for PublicInputValue {
        fn from(pi: PublicInput) -> PublicInputValue {
           match pi {
                PublicInput::BlsScalar(v) => PublicInputValue::from(v),
                PublicInput::JubJubScalar(v) => PublicInputValue::from(v),
                PublicInput::Point(v) => PublicInputValue::from(v),
            } 
        }
    }
}
