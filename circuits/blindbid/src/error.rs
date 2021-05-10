// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Error definitions for the BlindBid circuit & gadgets.

use core::fmt;
use dusk_blindbid::BlindBidError;
use dusk_bytes::Error as DuskBytesError;
use dusk_plonk::error::Error as PlonkError;
use dusk_poseidon::Error as PoseidonError;

#[derive(Debug)]
/// Compilation of the Error definitions for the BlindBid circuit & gadgets.
pub enum BlindBidCircuitError {
    /// Dusk-bytes serialization error
    SerializationError(DuskBytesError),
    /// Poseidon lib error
    PoseidonError(PoseidonError),
    /// Plonk lib error
    PlonkError(PlonkError),
    /// BlindBid lib error
    BlindBidError(BlindBidError),
}

impl fmt::Display for BlindBidCircuitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Bid Generation Error: {:?}", &self)
    }
}

impl From<BlindBidCircuitError> for std::io::Error {
    fn from(err: BlindBidCircuitError) -> std::io::Error {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("{:?}", err),
        )
    }
}

impl From<DuskBytesError> for BlindBidCircuitError {
    fn from(bytes_err: DuskBytesError) -> Self {
        Self::SerializationError(bytes_err)
    }
}

impl From<PoseidonError> for BlindBidCircuitError {
    fn from(poseidon_err: PoseidonError) -> Self {
        Self::PoseidonError(poseidon_err)
    }
}

impl From<PlonkError> for BlindBidCircuitError {
    fn from(plonk_err: PlonkError) -> Self {
        Self::PlonkError(plonk_err)
    }
}

impl From<BlindBidError> for BlindBidCircuitError {
    fn from(blindbid_err: BlindBidError) -> Self {
        Self::BlindBidError(blindbid_err)
    }
}
