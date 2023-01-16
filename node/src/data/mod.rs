// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::io::{self, Read, Write};

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum Topics {
    // Data exchange topics.
    GetData = 8,
    GetBlocks = 9,
    Tx = 10,
    Block = 11,
    MemPool = 13,
    Inv = 14,

    // Consensus main loop topics
    Candidate = 15,
    NewBlock = 16,
    Reduction = 17,

    // Consensus Agreement loop topics
    Agreement = 18,
    AggrAgreement = 19,

    Unknown = 100,
}

impl Default for Topics {
    fn default() -> Self {
        Topics::Unknown
    }
}

impl From<Topics> for u8 {
    fn from(t: Topics) -> Self {
        t as u8
    }
}

pub trait Serializable {
    fn write<W: Write>(&self, writer: &mut W) -> io::Result<()>;

    fn read<R: Read>(reader: &mut R) -> io::Result<Self>
    where
        Self: Sized;

    fn write_var_le_bytes<W: Write>(w: &mut W, buf: &[u8]) -> io::Result<()> {
        let len = buf.len() as u8;

        w.write_all(&len.to_le_bytes())?;
        w.write_all(buf)?;

        Ok(())
    }

    // read_var_le_bytes reads length-prefixed fields
    fn read_var_le_bytes<R: Read, const N: usize>(
        r: &mut R,
    ) -> io::Result<[u8; N]> {
        let mut buf = [0u8; 1];
        r.read_exact(&mut buf)?;

        debug_assert_eq!(buf[0] as usize, N);

        let mut buf = [0u8; N];
        r.read_exact(&mut buf)?;

        Ok(buf)
    }
}
