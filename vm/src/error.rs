// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::TxPreconditionError;
use dusk_core::transfer::PANIC_NONCE_NOT_READY;
use piecrust::Error;

/// Errors that can occur during transaction execution in the VM.
///
/// This enum encapsulates different types of errors that may arise, including
/// precondition failures, unspendable errors, and failed refunds.
/// Each variant provides context about the nature of the error, allowing for
/// more precise error handling and debugging.
#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    /// Occurs when a refund operation fails after transaction execution.
    #[error("Failed refund: {0}")]
    FailedRefund(Error),

    /// Occurs when the transaction is valid but cannot be processed at the
    /// moment, like when the nonce used is not the next one
    #[error("Nonce not ready to be used yet")]
    NotReady,

    /// Occurs when a precondition for transaction execution is not met.
    #[error("Precondition error: {0}")]
    Precondition(String),

    /// Occurs when a transaction is deemed unspendable due to an error during
    /// execution.
    #[error("Unspendable: {0}")]
    Unspendable(Error),
}

impl ExecutionError {
    /// Creates a new `ExecutionError` with the `Precondition` variant.
    pub fn precondition<T: ToString>(msg: T) -> Self {
        Self::Precondition(msg.to_string())
    }

    /// Creates a new `ExecutionError` from an existing `Error` happening during
    /// the spend_or_execute phase, categorizing it as `Unspendable` unless it's
    /// a specific nonce not ready panic.
    pub fn from_spend_and_execute(inner: Error) -> Self {
        if let Error::Panic(val) = &inner
            && val == PANIC_NONCE_NOT_READY
        {
            Self::NotReady
        } else {
            Self::Unspendable(inner)
        }
    }
    /// Creates a new `ExecutionError` with the `FailedRefund` variant.
    pub fn failed_refund(inner: Error) -> Self {
        Self::FailedRefund(inner)
    }
}

impl From<TxPreconditionError> for ExecutionError {
    fn from(err: TxPreconditionError) -> Self {
        ExecutionError::Precondition(err.legacy_to_string())
    }
}
