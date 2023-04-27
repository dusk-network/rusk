// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use node_data::message::Message;
use node_data::Serializable;
use std::io::{self, Read, Write};

const RROTOCOL_VERSION: [u8; 8] = [0, 0, 0, 0, 1, 0, 0, 0];

/// Defines PDU (Protocol Data Unit) structure.
#[derive(Debug, Default)]
pub struct Pdu {
    pub header: Header,
    pub payload: node_data::message::Message,
}

/// Frame Header definition.
#[derive(Debug, Default)]
pub struct Header {
    version: [u8; 8],
    reserved: u64,
    checksum: [u8; 4],
}

impl Pdu {
    pub fn encode(msg: &Message) -> io::Result<Vec<u8>> {
        let mut payload_buf = vec![];
        msg.write(&mut payload_buf)?;

        let mut header_buf = vec![];
        Header {
            checksum: calc_checksum(&payload_buf[..]),
            version: RROTOCOL_VERSION,
            reserved: 0,
        }
        .write(&mut header_buf)?;

        let frame_size = (header_buf.len() + payload_buf.len()) as u64;

        Ok(
            [Vec::from(frame_size.to_le_bytes()), header_buf, payload_buf]
                .concat(),
        )
    }

    pub fn decode<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;

        let header = Header::read(r)?;
        let payload = Message::read(r)?;

        Ok(Pdu { header, payload })
    }
}

impl Serializable for Header {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&self.version[..])?;
        w.write_all(&self.reserved.to_le_bytes())?;
        w.write_all(&self.checksum[..])?;
        Ok(())
    }

    /// Deserialize struct from buf by consuming N bytes.
    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let mut version = [0u8; 8];
        r.read_exact(&mut version)?;

        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;
        let reserved = u64::from_le_bytes(buf);

        let mut checksum = [0u8; 4];
        r.read_exact(&mut checksum)?;

        Ok(Header {
            version,
            reserved,
            checksum,
        })
    }
}

fn calc_checksum(buf: &[u8]) -> [u8; 4] {
    use blake2::{digest::consts::U32, Blake2b, Digest};

    let mut h = Blake2b::<U32>::new();
    h.update(buf);
    let res = h.finalize();

    let mut v = [0u8; 4];
    v.clone_from_slice(&res[0..4]);
    v
}
