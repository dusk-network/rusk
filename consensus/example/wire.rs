// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bytes::Bytes;
use consensus::messages::{Message, Serializable2};
use std::io::{self, Read, Write};

/// Wire Frame definition.
#[derive(Debug, Default)]
pub struct Frame {
    header: FrameHeader,
    payload: FramePayload,
}

/// Frame Header definition.
#[derive(Debug, Default)]
struct FrameHeader {
    version: u64,
    reserved: u64,
    checksum: u32,
}

/// Frame Payload definition.
#[derive(Debug, Default)]
struct FramePayload(Message);

impl Serializable2 for FrameHeader {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        w.write_all(&self.version.to_le_bytes())?;
        w.write_all(&self.reserved.to_le_bytes())?;
        w.write_all(&self.checksum.to_le_bytes())?;
        Ok(())
    }

    /// Deserialize struct from buf by consuming N bytes.
    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;
        let version = u64::from_le_bytes(buf);

        let mut buf = [0u8; 8];
        r.read_exact(&mut buf)?;
        let reserved = u64::from_le_bytes(buf);

        let mut buf = [0u8; 4];
        r.read_exact(&mut buf)?;
        let checksum = u32::from_le_bytes(buf);

        Ok(FrameHeader {
            version,
            reserved,
            checksum,
        })
    }
}

impl Serializable2 for Frame {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        let mut buf = vec![];
        self.header.write(&mut buf)?;
        self.payload.0.write(&mut buf)?;

        let frame_size = buf.len() as u64;

        w.write_all(&frame_size.to_le_bytes())?;
        w.write_all(&buf[..])?;

        Ok(())
    }

    /// Deserialize struct from buf by consuming N bytes.
    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let mut buf = [0u8; 8];
        _ = r.read_exact(&mut buf)?;

        let header = FrameHeader::read(r)?;
        let payload = FramePayload(Message::read(r)?);

        Ok(Frame { header, payload })
    }
}

impl Frame {
    pub fn encode(msg: Message) -> io::Result<Vec<u8>> {
        let mut buf = vec![];

        Self {
            header: FrameHeader::default(),
            payload: FramePayload(msg),
        }
        .write(&mut buf)?;

        Ok(buf)
    }

    pub fn decode(bytes: Vec<u8>) -> io::Result<Self> {
        Frame::read(&mut &bytes[..])
    }

    pub fn get_topic(&self) -> u8 {
        self.payload.0.header.topic
    }

    pub fn get_msg(&self) -> &Message {
        &self.payload.0
    }
}
