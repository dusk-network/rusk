// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod encoding;
pub mod ledger;

use std::io::{self, Read, Write};

pub trait Serializable {
    fn write<W: Write>(&self, writer: &mut W) -> io::Result<()>;
    fn read<R: Read>(reader: &mut R) -> io::Result<Self>
    where
        Self: Sized;

    /// Writes length-prefixed fields
    fn write_var_le_bytes<W: Write>(w: &mut W, buf: &[u8]) -> io::Result<()> {
        let len = buf.len() as u8;
        w.write_all(&len.to_le_bytes())?;
        w.write_all(buf)?;
        Ok(())
    }

    /// Reads length-prefixed fields
    fn read_var_le_bytes<R: Read>(r: &mut R) -> io::Result<Vec<u8>> {
        let mut buf = [0u8; 1];
        r.read_exact(&mut buf)?;

        let mut buf = vec![0u8; buf[0] as usize];
        r.read_exact(&mut buf)?;

        Ok(buf)
    }
}
