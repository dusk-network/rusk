// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod bls;
pub mod encoding;
pub mod ledger;
pub mod message;

use std::io::{self, Read, Write};

pub trait Serializable {
    fn write<W: Write>(&self, writer: &mut W) -> io::Result<()>;
    fn read<R: Read>(reader: &mut R) -> io::Result<Self>
    where
        Self: Sized;

    /// Write varint fields
    fn write_varint<W: Write>(w: &mut W, v: u64) -> io::Result<()> {
        if v < 0xfd {
            w.write_all(&[v as u8])?
        } else if v <= 1 << 16 - 1 {
            w.write_all(&[0xfd])?;
            w.write_all(&(v as u16).to_le_bytes())?;
        } else if v <= 1 << 32 - 1 {
            w.write_all(&[0xfe])?;
            w.write_all(&(v as u32).to_le_bytes())?;
        } else {
            w.write_all(&[0xff])?;
            w.write_all(&v.to_le_bytes())?;
        }
        Ok(())
    }

    /// Write length-prefixed fields
    fn write_var_bytes<W: Write>(w: &mut W, buf: &[u8]) -> io::Result<()> {
        Self::write_varint(w, buf.len() as u64)?;
        w.write_all(buf)
    }

    /// Reads varint fields
    fn read_varint<R: Read>(r: &mut R) -> io::Result<usize> {
        let mut buf = [0u8; 1];
        r.read_exact(&mut buf)?;
        let d = buf[0];

        let len = match d {
            0xff => {
                let mut buf = [0u8; 8];
                r.read_exact(&mut buf)?;
                u64::from_le_bytes(buf) as usize
            }
            0xfe => {
                let mut buf = [0u8; 4];
                r.read_exact(&mut buf)?;
                u32::from_le_bytes(buf) as usize
            }
            0xfd => {
                let mut buf = [0u8; 2];
                r.read_exact(&mut buf)?;
                u16::from_le_bytes(buf) as usize
            }
            val => val as usize,
        };

        Ok(len)
    }

    /// Reads length-prefixed fields
    fn read_var_bytes<R: Read>(r: &mut R) -> io::Result<Vec<u8>> {
        let len = Self::read_varint(r)?;

        let mut buf = vec![0u8; len];
        r.read_exact(&mut buf)?;

        Ok(buf)
    }

    /// Writes length-prefixed fields
    fn write_var_le_bytes<W: Write>(w: &mut W, buf: &[u8]) -> io::Result<()> {
        let len = buf.len() as u64;
        w.write_all(&len.to_le_bytes())?;
        w.write_all(buf)?;
        Ok(())
    }

    /// Reads length-prefixed fields
    fn read_var_le_bytes<R: Read>(r: &mut R) -> io::Result<Vec<u8>> {
        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;
        let len = u64::from_le_bytes(buf) as usize;

        let mut buf = vec![0u8; len];
        r.read_exact(&mut buf)?;

        Ok(buf)
    }

    /// Writes length-prefixed fields
    fn write_var_le_bytes32<W: Write>(w: &mut W, buf: &[u8]) -> io::Result<()> {
        let len = buf.len() as u32;
        w.write_all(&len.to_le_bytes())?;
        w.write_all(buf)?;
        Ok(())
    }

    /// Reads length-prefixed fields
    fn read_var_le_bytes32<R: Read>(r: &mut R) -> io::Result<Vec<u8>> {
        let mut buf = [0u8; 4];
        r.read_exact(&mut buf)?;
        let len = u32::from_le_bytes(buf) as usize;

        let mut buf = vec![0u8; len];
        r.read_exact(&mut buf)?;

        Ok(buf)
    }
}
