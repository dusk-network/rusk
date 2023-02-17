// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::Serializable;
use dusk_bytes::Serializable as DuskBytesSerializable;
use sha3::Digest;
use std::io::{self, Read, Write};

pub type Seed = Signature;
pub type Hash = [u8; 32];

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub header: Header,
    pub txs: Vec<Transaction>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Header {
    // Hashable fields
    pub version: u8,
    pub height: u64,
    pub timestamp: i64,
    pub prev_block_hash: Hash,
    pub seed: Seed,
    pub state_hash: Hash,
    pub generator_bls_pubkey: BlsPubkey,
    pub gas_limit: u64,

    // Block hash
    pub hash: Hash,

    // Non-hashable fields
    pub cert: Certificate,
}

#[derive(Debug, Clone)]
pub struct Transaction {
    pub inner: dusk_wallet_core::Transaction,
    pub gas_spent: Option<u64>,
}

impl Transaction {
    pub fn hash(&self) -> [u8; 32] {
        self.inner.hash().to_bytes().into()
    }
    pub fn gas_price(&self) -> u64 {
        self.inner.fee().gas_price
    }
}

impl PartialEq for Transaction {
    fn eq(&self, other: &Self) -> bool {
        todo!()
    }

    fn ne(&self, other: &Self) -> bool {
        todo!()
    }
}

impl Eq for Transaction {}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Certificate {
    pub first_reduction: StepVotes,
    pub second_reduction: StepVotes,
    pub step: u8,
}

impl Header {
    /// Marshal hashable fields.
    ///
    /// Param `fixed_size_seed` changes the way seed is marshaled.
    /// In block hashing, header seed is fixed-size field while in wire
    /// message marshaling it is variable-length field.
    pub(crate) fn marshal_hashable<W: Write>(
        &self,
        w: &mut W,
        fixed_size_seed: bool,
    ) -> io::Result<()> {
        w.write_all(&self.version.to_le_bytes())?;
        w.write_all(&self.height.to_le_bytes())?;
        w.write_all(&(self.timestamp as u64).to_le_bytes())?;
        w.write_all(&self.prev_block_hash[..])?;

        if fixed_size_seed {
            w.write_all(&self.seed.inner()[..])?;
        } else {
            Self::write_var_le_bytes(w, &self.seed.inner()[..])?;
        }

        w.write_all(&self.state_hash[..])?;
        w.write_all(&self.generator_bls_pubkey.inner()[..])?;
        w.write_all(&self.gas_limit.to_le_bytes())?;

        Ok(())
    }

    pub(crate) fn unmarshal_hashable<R: Read>(r: &mut R) -> io::Result<Self> {
        let mut buf = [0u8; 1];
        r.read_exact(&mut buf[..])?;
        let version = buf[0];

        let mut buf = [0u8; 8];
        r.read_exact(&mut buf[..])?;
        let height = u64::from_le_bytes(buf);

        let mut buf = [0u8; 8];
        r.read_exact(&mut buf[..])?;
        let timestamp = u64::from_le_bytes(buf) as i64;

        let mut prev_block_hash = [0u8; 32];
        r.read_exact(&mut prev_block_hash[..])?;

        let value = Self::read_var_le_bytes(r)?;
        let seed: [u8; 48] = value
            .try_into()
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidData))?;

        let mut state_hash = [0u8; 32];
        r.read_exact(&mut state_hash[..])?;

        let mut generator_bls_pubkey = [0u8; 96];
        r.read_exact(&mut generator_bls_pubkey[..])?;

        let mut buf = [0u8; 8];
        r.read_exact(&mut buf[..])?;
        let gas_limit = u64::from_le_bytes(buf);

        Ok(Header {
            version,
            height,
            timestamp,
            gas_limit,
            prev_block_hash,
            seed: Seed::from(seed),
            generator_bls_pubkey: BlsPubkey(generator_bls_pubkey),
            state_hash,
            hash: [0; 32],
            cert: Default::default(),
        })
    }
}

impl Block {
    /// Creates a new block and calculates block hash, if missing.
    pub fn new(header: Header, txs: Vec<Transaction>) -> io::Result<Self> {
        let mut b = Block { header, txs };
        b.calculate_hash()?;
        Ok(b)
    }

    pub fn calculate_hash(&mut self) -> io::Result<()> {
        // Call hasher only if header.hash is empty
        if self.header.hash != Hash::default() {
            return Ok(());
        }

        let mut hasher = sha3::Sha3_256::new();
        self.header.marshal_hashable(&mut hasher, true)?;

        self.header.hash = hasher.finalize().into();
        Ok(())
    }
}

#[derive(Debug, Default, Clone, Eq, Hash, PartialEq)]
pub struct StepVotes {
    pub bitset: u64,
    pub signature: Signature,
}

impl StepVotes {
    pub fn new(signature: [u8; 48], bitset: u64) -> StepVotes {
        StepVotes {
            bitset,
            signature: Signature(signature),
        }
    }
}

/// a wrapper of 48-sized array to facilitate Signature
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub struct Signature(pub [u8; 48]);

impl Signature {
    pub fn is_zeroed(&self) -> bool {
        self.0 == [0; 48]
    }
    pub fn inner(&self) -> [u8; 48] {
        self.0
    }
}

impl From<[u8; 48]> for Signature {
    fn from(value: [u8; 48]) -> Self {
        Self(value)
    }
}

impl Default for Signature {
    fn default() -> Self {
        Signature([0; 48])
    }
}

/// a wrapper of 96-sized array to facilitate BLS Public key
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub struct BlsPubkey(pub [u8; 96]);

impl Default for BlsPubkey {
    fn default() -> Self {
        BlsPubkey([0; 96])
    }
}

impl BlsPubkey {
    pub fn inner(&self) -> [u8; 96] {
        self.0
    }
}
