// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Expose the `ErrorCode` enum to be used in FFI exported functions.
//! ErrorCode enum represents different error codes for FFI, mapped to u8
//! values.
//! The Ok variant signifies successful execution.

use core::ops::{ControlFlow, FromResidual, Try};

/// [`ErrorCode`] enum represents different error codes for FFI, mapped to
/// [`u8`] values.
/// The [`Ok`] variant signifies successful execution.
#[derive(Debug, Clone)]
#[repr(u8)]
pub enum ErrorCode {
    //  Archiving (rkyv serialization) error
    ArchivingError = 255,
    // Unarchiving (rkyv deserialization) error
    UnarchivingError = 254,
    // Deserialization (dusk-bytes deserialization) error
    DeserializationError = 253,
    // Success
    Ok = 0,
}

impl Try for ErrorCode {
    type Output = ErrorCode;
    type Residual = ErrorCode;

    fn from_output(_: Self::Output) -> Self {
        ErrorCode::Ok
    }

    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            ErrorCode::Ok => ControlFlow::Continue(ErrorCode::Ok), /* Continue execution on success */
            _ => ControlFlow::Break(self), /* Return the error code early */
        }
    }
}

impl FromResidual<ErrorCode> for ErrorCode {
    fn from_residual(residual: ErrorCode) -> Self {
        residual // Simply return the error code as is
    }
}

impl FromResidual<Result<core::convert::Infallible, ErrorCode>> for ErrorCode {
    fn from_residual(
        residual: Result<core::convert::Infallible, ErrorCode>,
    ) -> Self {
        match residual {
            Err(e) => e,
            _ => unreachable!(),
        }
    }
}
