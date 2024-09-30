// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use serde::Serialize;

use crate::message::{ConsensusHeader, MESSAGE_MAX_ITER};

use super::*;

pub type Seed = Signature;
#[derive(Eq, PartialEq, Clone, Serialize)]
#[cfg_attr(any(feature = "faker", test), derive(Dummy))]
pub struct Header {
    // Hashable fields
    pub version: u8,
    pub height: u64,
    pub timestamp: u64,
    #[serde(serialize_with = "crate::serialize_hex")]
    pub prev_block_hash: Hash,
    pub seed: Seed,
    #[serde(serialize_with = "crate::serialize_hex")]
    pub state_hash: Hash,
    #[serde(serialize_with = "crate::serialize_hex")]
    pub event_bloom: Bloom,
    pub generator_bls_pubkey: PublicKeyBytes,
    #[serde(serialize_with = "crate::serialize_hex")]
    pub txroot: Hash,
    #[serde(serialize_with = "crate::serialize_hex")]
    pub faultroot: Hash,
    pub gas_limit: u64,
    #[cfg_attr(any(feature = "faker", test), dummy(faker = "0..50"))]
    pub iteration: u8,
    pub prev_block_cert: Attestation,
    pub failed_iterations: IterationsInfo,

    // Block hash
    #[serde(serialize_with = "crate::serialize_hex")]
    pub hash: Hash,

    pub signature: Signature,

    // Non-hashable fields
    #[serde(skip_serializing)]
    pub att: Attestation,
}

impl Default for Header {
    fn default() -> Self {
        Self {
            version: Default::default(),
            height: Default::default(),
            timestamp: Default::default(),
            prev_block_hash: Default::default(),
            seed: Default::default(),
            state_hash: Default::default(),
            event_bloom: [0u8; 256],
            generator_bls_pubkey: Default::default(),
            txroot: Default::default(),
            faultroot: Default::default(),
            gas_limit: Default::default(),
            iteration: Default::default(),
            prev_block_cert: Default::default(),
            failed_iterations: Default::default(),
            hash: Default::default(),
            signature: Default::default(),
            att: Default::default(),
        }
    }
}

impl Header {
    pub fn size(&self) -> io::Result<usize> {
        let mut buf = vec![];
        self.write(&mut buf)?;
        Ok(buf.len())
    }
}

impl std::fmt::Debug for Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let timestamp =
            chrono::DateTime::from_timestamp(self.timestamp as i64, 0)
                .map_or_else(|| "unknown".to_owned(), |v| v.to_rfc2822());

        f.debug_struct("Header")
            .field("version", &self.version)
            .field("height", &self.height)
            .field("timestamp", &timestamp)
            .field("prev_block_hash", &to_str(&self.prev_block_hash))
            .field("seed", &to_str(self.seed.inner()))
            .field("state_hash", &to_str(&self.state_hash))
            .field("event_hash", &to_str(&self.event_bloom))
            .field("gen_bls_pubkey", &to_str(self.generator_bls_pubkey.inner()))
            .field("gas_limit", &self.gas_limit)
            .field("hash", &to_str(&self.hash))
            .field("signature", &to_str(self.signature.inner()))
            .field("att", &self.att)
            .field("tx_root", &to_str(&self.txroot))
            .field("fault_root", &to_str(&self.faultroot))
            .finish()
    }
}

impl Header {
    /// Return the corresponding ConsensusHeader
    pub fn to_consensus_header(&self) -> ConsensusHeader {
        ConsensusHeader {
            prev_block_hash: self.prev_block_hash,
            round: self.height,
            iteration: self.iteration,
        }
    }

    /// Marshal hashable fields.
    pub(crate) fn marshal_hashable<W: Write>(
        &self,
        w: &mut W,
    ) -> io::Result<()> {
        w.write_all(&self.version.to_le_bytes())?;
        w.write_all(&self.height.to_le_bytes())?;
        w.write_all(&self.timestamp.to_le_bytes())?;
        w.write_all(&self.prev_block_hash)?;

        w.write_all(self.seed.inner())?;

        w.write_all(&self.state_hash)?;
        w.write_all(&self.event_bloom)?;
        w.write_all(self.generator_bls_pubkey.inner())?;
        w.write_all(&self.txroot)?;
        w.write_all(&self.faultroot)?;
        w.write_all(&self.gas_limit.to_le_bytes())?;
        w.write_all(&self.iteration.to_le_bytes())?;
        self.prev_block_cert.write(w)?;
        self.failed_iterations.write(w)?;

        Ok(())
    }

    pub(crate) fn unmarshal_hashable<R: Read>(r: &mut R) -> io::Result<Self> {
        let version = Self::read_u8(r)?;
        let height = Self::read_u64_le(r)?;
        let timestamp = Self::read_u64_le(r)?;

        let prev_block_hash = Self::read_bytes(r)?;
        let seed = Self::read_bytes(r)?;
        let state_hash = Self::read_bytes(r)?;
        let event_hash = Self::read_bytes(r)?;
        let generator_bls_pubkey = Self::read_bytes(r)?;
        let txroot = Self::read_bytes(r)?;
        let faultroot = Self::read_bytes(r)?;
        let gas_limit = Self::read_u64_le(r)?;
        let iteration = Self::read_u8(r)?;

        // Iteration is 0-based
        if iteration >= MESSAGE_MAX_ITER {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid iteration {iteration})"),
            ));
        }

        let prev_block_cert = Attestation::read(r)?;
        let failed_iterations = IterationsInfo::read(r)?;

        Ok(Header {
            version,
            height,
            timestamp,
            gas_limit,
            prev_block_hash,
            seed: Seed::from(seed),
            generator_bls_pubkey: PublicKeyBytes(generator_bls_pubkey),
            iteration,
            state_hash,
            event_bloom: event_hash,
            txroot,
            faultroot,
            hash: [0; 32],
            att: Default::default(),
            prev_block_cert,
            failed_iterations,
            signature: Default::default(),
        })
    }
}
