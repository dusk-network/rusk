// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bytes::{Buf, BufMut, Bytes, BytesMut};
use consensus::messages::{Message, Serializable};
use std::mem;

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

impl Serializable for FrameHeader {
    fn to_bytes(&self) -> Vec<u8> {
        let mut buf = BytesMut::with_capacity(mem::size_of::<FrameHeader>());
        buf.put_u64_le(self.version);
        buf.put_u64_le(self.reserved);
        buf.put_u32_le(self.checksum);
        buf.to_vec()
    }

    fn from_bytes(buf: &mut Bytes) -> Self {
        Self {
            version: buf.get_u64_le(),
            reserved: buf.get_u64_le(),
            checksum: buf.get_u32_le(),
        }
    }
}

impl FramePayload {
    fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }

    fn from_bytes(buf: &mut Bytes) -> Self {
        Self(Message::from_bytes(buf))
    }
}

impl Serializable for Frame {
    fn to_bytes(&self) -> Vec<u8> {
        let header_as_vec = self.header.to_bytes();
        let payload_as_vec = self.payload.to_bytes();

        let frame_size = header_as_vec.len() + payload_as_vec.len();

        let mut buf = BytesMut::with_capacity(8 + frame_size);

        buf.put_u64_le(frame_size as u64);
        buf.put(&header_as_vec[..]);
        buf.put(&payload_as_vec[..]);

        buf.to_vec()
    }

    fn from_bytes(buf: &mut Bytes) -> Self {
        _ = buf.get_u64_le();

        Self {
            header: FrameHeader::from_bytes(buf),
            payload: FramePayload::from_bytes(buf),
        }
    }
}

impl Frame {
    pub fn encode(msg: Message) -> Vec<u8> {
        Self {
            header: FrameHeader::default(),
            payload: FramePayload(msg),
        }
        .to_bytes()
    }

    pub fn decode(bytes: Vec<u8>) -> Self {
        Frame::from_bytes(&mut Bytes::from(bytes))
    }

    pub fn get_topic(&self) -> u8 {
        self.payload.0.header.topic
    }

    pub fn get_msg(&self) -> &Message {
        &self.payload.0
    }
}
