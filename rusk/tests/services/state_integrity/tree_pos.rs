// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

use std::collections::BTreeMap;
use std::io;
use std::io::{ErrorKind, Read};

#[derive(Debug, Clone, Default, Archive, Deserialize, Serialize)]
#[archive_attr(derive(CheckBytes))]
pub struct TreePos {
    tree_pos: BTreeMap<u32, ([u8; 32], u64)>,
}

impl TreePos {
    pub fn insert(&mut self, k: u32, v: ([u8; 32], u64)) {
        self.tree_pos.insert(k, v);
    }

    fn read_bytes<R: Read, const N: usize>(r: &mut R) -> io::Result<[u8; N]> {
        let mut buffer = [0u8; N];
        r.read_exact(&mut buffer)?;
        Ok(buffer)
    }

    fn is_eof<T>(r: &io::Result<T>) -> bool {
        if let Err(ref e) = r {
            if e.kind() == ErrorKind::UnexpectedEof {
                return true;
            }
        }
        false
    }

    pub fn unmarshall<R: Read>(r: &mut R) -> io::Result<Self> {
        let mut slf = Self::default();
        loop {
            let res = Self::read_bytes(r);
            if Self::is_eof(&res) {
                break;
            }
            let k = u32::from_le_bytes(res?);

            let res = Self::read_bytes(r);
            if Self::is_eof(&res) {
                break;
            }
            let hash: [u8; 32] = res?;

            let res = Self::read_bytes(r);
            if Self::is_eof(&res) {
                break;
            }
            let p = u32::from_le_bytes(res?);
            slf.tree_pos.insert(k, (hash, p as u64));
        }
        Ok(slf)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&u32, &([u8; 32], u64))> {
        self.tree_pos.iter()
    }
}
