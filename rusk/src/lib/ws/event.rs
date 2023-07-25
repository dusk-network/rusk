// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use serde::Deserialize;

/// A request sent by the websocket client.
#[derive(Debug, Deserialize)]
pub(crate) struct Request {
    pub(crate) headers: serde_json::Map<String, serde_json::Value>,
    pub(crate) target_type: u8,
    pub(crate) target: String,
    pub(crate) topic: String,
    #[serde(skip)]
    pub(crate) binary_data: Vec<u8>,
    pub(crate) data: String,
}

impl Request {
    pub fn parse(bytes: &[u8]) -> anyhow::Result<Self> {
        let a: Vec<u8> = vec![];
        let (headers, bytes) = parse_header(bytes)?;
        let (target_type, bytes) = parse_target_type(bytes)?;
        let (target, bytes) = parse_string(bytes)?;
        let (topic, bytes) = parse_string(bytes)?;
        let data = bytes.to_vec();
        Ok(Self {
            headers,
            target_type,
            target,
            topic,
            binary_data: data,
            data: "".into(),
        })
    }
}

fn parse_len(bytes: &[u8]) -> anyhow::Result<(usize, &[u8])> {
    if bytes.len() < 4 {
        return Err(anyhow::anyhow!("not enough bytes"));
    }

    let len =
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    let (_, left) = bytes.split_at(len);

    Ok((len, left))
}

type Header<'a> = (serde_json::Map<String, serde_json::Value>, &'a [u8]);
fn parse_header(bytes: &[u8]) -> anyhow::Result<Header> {
    let (len, bytes) = parse_len(bytes)?;
    if bytes.len() < len {
        return Err(anyhow::anyhow!("not enough bytes for parsed len {len}"));
    }

    let (header_bytes, bytes) = bytes.split_at(len);
    let header = serde_json::from_slice(header_bytes)?;

    Ok((header, bytes))
}

fn parse_target_type(bytes: &[u8]) -> anyhow::Result<(u8, &[u8])> {
    if bytes.is_empty() {
        return Err(anyhow::anyhow!("not enough bytes for target type"));
    }

    let (target_type_bytes, bytes) = bytes.split_at(1);
    let target_type = target_type_bytes[0];

    Ok((target_type, bytes))
}

fn parse_string(bytes: &[u8]) -> anyhow::Result<(String, &[u8])> {
    let (len, bytes) = parse_len(bytes)?;
    if bytes.len() < len {
        return Err(anyhow::anyhow!("not enough bytes for parsed len {len}"));
    }

    let (string_bytes, bytes) = bytes.split_at(len);
    let string = String::from_utf8(string_bytes.to_vec())?;

    Ok((string, bytes))
}

#[allow(unused)]
struct StateQuery {}
#[allow(unused)]
struct ChainQuery {}
