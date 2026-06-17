// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::*;
impl<DB: DBAccess> Metadata for DBTransaction<'_, DB> {
    fn op_write<T: AsRef<[u8]>>(&mut self, key: &[u8], value: T) -> Result<()> {
        self.put_cf(self.metadata_cf, key, value)?;
        Ok(())
    }

    fn op_read(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        self.inner.get_cf(self.metadata_cf, key).map_err(Into::into)
    }
}

pub(super) fn serialize_key(
    value: u64,
    hash: [u8; 32],
) -> std::io::Result<Vec<u8>> {
    let mut w = vec![];
    std::io::Write::write_all(&mut w, &value.to_be_bytes())?;
    std::io::Write::write_all(&mut w, &hash)?;
    Ok(w)
}

pub(super) fn deserialize_key<R: Read>(r: &mut R) -> Result<(u64, [u8; 32])> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    let value = u64::from_be_bytes(buf);
    let mut hash = [0u8; 32];
    r.read_exact(&mut hash[..])?;

    Ok((value, hash))
}

pub(super) fn serialize_iter_key(
    ch: &ConsensusHeader,
) -> std::io::Result<Vec<u8>> {
    let mut w = vec![];
    std::io::Write::write_all(&mut w, &ch.prev_block_hash)?;
    std::io::Write::write_all(&mut w, &[ch.iteration])?;
    Ok(w)
}

pub(super) fn deserialize_iter_key<R: Read>(
    r: &mut R,
) -> Result<([u8; 32], u8)> {
    let mut prev_block_hash = [0u8; 32];
    r.read_exact(&mut prev_block_hash)?;

    let mut iter_byte = [0u8; 1];
    r.read_exact(&mut iter_byte)?;
    let iteration = u8::from_be_bytes(iter_byte);

    Ok((prev_block_hash, iteration))
}
