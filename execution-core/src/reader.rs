// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Functions for reading variable length elements from bytes.

extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

use dusk_bytes::Error::InvalidData;
use dusk_bytes::{DeserializableSlice, Error as BytesError};

/// Reads vector from a buffer.
/// Resets buffer to a position after the bytes read.
///
/// # Errors
/// When length or data could not be read.
pub fn read_vec(buf: &mut &[u8]) -> Result<Vec<u8>, BytesError> {
    let len = usize::try_from(u64::from_reader(buf)?)
        .map_err(|_| BytesError::InvalidData)?;
    if buf.len() < len {
        return Err(InvalidData);
    }
    let bytes = buf[..len].into();
    *buf = &buf[len..];
    Ok(bytes)
}

/// Reads string from a buffer.
/// Resets buffer to a position after the bytes read.
///
/// # Errors
/// When length or data could not be read.
pub fn read_str(buf: &mut &[u8]) -> Result<String, BytesError> {
    let len = usize::try_from(u64::from_reader(buf)?)
        .map_err(|_| BytesError::InvalidData)?;
    if buf.len() < len {
        return Err(InvalidData);
    }
    let str = String::from_utf8(buf[..len].into())
        .map_err(|_| BytesError::InvalidData)?;
    *buf = &buf[len..];
    Ok(str)
}

/// Reads array from a buffer.
/// Resets buffer to a position after the bytes read.
///
/// # Errors
/// When length or data could not be read.
pub fn read_arr<const N: usize>(
    buf: &mut &[u8],
) -> Result<[u8; N], BytesError> {
    if buf.len() < N {
        return Err(InvalidData);
    }
    let mut a = [0u8; N];
    a.copy_from_slice(&buf[..N]);
    *buf = &buf[N..];
    Ok(a)
}
