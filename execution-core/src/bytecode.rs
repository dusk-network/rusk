// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Wrapper for a long data that we want to keep the integrity of.

extern crate alloc;
use alloc::vec::Vec;
use bytecheck::CheckBytes;
use dusk_bytes::Error as BytesError;
use dusk_bytes::Error::InvalidData;
use rkyv::{Archive, Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
/// Holds bytes of bytecode and its hash.
pub struct Bytecode {
    /// Hash of the bytecode bytes.
    pub hash: [u8; 32],
    /// Bytecode bytes.
    pub bytes: Vec<u8>,
}

impl Bytecode {
    /// Provides contribution bytes for an external hash.
    #[must_use]
    pub fn to_hash_input_bytes(&self) -> Vec<u8> {
        self.hash.to_vec()
    }

    /// Serializes this object into a variable length buffer
    #[must_use]
    pub fn to_var_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(self.hash);
        bytes
    }

    /// Deserialize from a bytes buffer.
    ///
    /// # Errors
    /// Errors when the bytes are not available.
    pub fn from_buf(buf: &[u8]) -> Result<(Self, usize), BytesError> {
        if buf.len() < 32 {
            return Err(InvalidData);
        }
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&buf[..32]);
        Ok((
            Self {
                hash,
                bytes: Vec::new(),
            },
            32,
        ))
    }
}
