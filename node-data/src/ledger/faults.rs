// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::message::{
    payload::{Candidate, Ratification, Validation, Vote},
    ConsensusHeader, SignInfo, StepMessage,
};

use dusk_bytes::Serializable as DuskSerializeble;
use execution_core::{BlsAggPublicKey, BlsSigError, BlsSignature};

use super::*;

#[derive(Debug, Clone)]
#[cfg_attr(any(feature = "faker", test), derive(fake::Dummy, Eq, PartialEq))]
pub enum Fault {
    DoubleCandidate(FaultData<Hash>, FaultData<Hash>),
    DoubleRatificationVote(FaultData<Vote>, FaultData<Vote>),
    DoubleValidationVote(FaultData<Vote>, FaultData<Vote>),
}

impl Fault {
    /// Hash the serialized form
    pub fn hash(&self) -> [u8; 32] {
        let mut b = vec![];
        self.write(&mut b).expect("Write to a vec shall not fail");
        Hasher::digest(b).to_bytes()
    }

    /// Return the IDs of the inner faults.
    pub fn ids(&self) -> (Hash, Hash) {
        match self {
            Fault::DoubleCandidate(a, b) => {
                let seed = Candidate::SIGN_SEED;
                let a = Hasher::digest(a.get_signed_data(seed)).to_bytes();
                let b = Hasher::digest(b.get_signed_data(seed)).to_bytes();
                (a, b)
            }
            Fault::DoubleRatificationVote(a, b) => {
                let seed = Ratification::SIGN_SEED;
                let a = Hasher::digest(a.get_signed_data(seed)).to_bytes();
                let b = Hasher::digest(b.get_signed_data(seed)).to_bytes();
                (a, b)
            }
            Fault::DoubleValidationVote(a, b) => {
                let seed = Validation::SIGN_SEED;
                let a = Hasher::digest(a.get_signed_data(seed)).to_bytes();
                let b = Hasher::digest(b.get_signed_data(seed)).to_bytes();
                (a, b)
            }
        }
    }

    /// Get the ConsensusHeader related to the inner FaultDatas
    pub fn consensus_header(&self) -> (&ConsensusHeader, &ConsensusHeader) {
        match self {
            Fault::DoubleRatificationVote(a, b)
            | Fault::DoubleValidationVote(a, b) => (&a.header, &b.header),
            Fault::DoubleCandidate(a, b) => (&a.header, &b.header),
        }
    }

    /// Check if both faults are signed properly
    pub fn verify_sigs(&self) -> Result<(), BlsSigError> {
        match self {
            Fault::DoubleCandidate(a, b) => {
                let seed = Candidate::SIGN_SEED;
                let msg = a.get_signed_data(seed);
                Self::verify_signature(&a.sig, &msg)?;
                let msg = b.get_signed_data(seed);
                Self::verify_signature(&b.sig, &msg)?;
                Ok(())
            }
            Fault::DoubleRatificationVote(a, b) => {
                let seed = Ratification::SIGN_SEED;
                let msg = a.get_signed_data(seed);
                Self::verify_signature(&a.sig, &msg)?;
                let msg = b.get_signed_data(seed);
                Self::verify_signature(&b.sig, &msg)?;
                Ok(())
            }
            Fault::DoubleValidationVote(a, b) => {
                let seed = Validation::SIGN_SEED;
                let msg = a.get_signed_data(seed);
                Self::verify_signature(&a.sig, &msg)?;
                let msg = b.get_signed_data(seed);
                Self::verify_signature(&b.sig, &msg)?;
                Ok(())
            }
        }
    }

    fn verify_signature(
        sign_info: &SignInfo,
        msg: &[u8],
    ) -> Result<(), BlsSigError> {
        let signature = sign_info.signature.inner();
        let sig = BlsSignature::from_bytes(signature)?;
        let pk = BlsAggPublicKey::from(sign_info.signer.inner());
        pk.verify(&sig, msg)
    }
}

impl FaultData<Hash> {
    fn get_signed_data(&self, seed: &[u8]) -> Vec<u8> {
        let mut signable = self.header.signable();
        signable.extend_from_slice(seed);
        signable.extend_from_slice(&self.data);
        signable
    }
}
impl FaultData<Vote> {
    fn get_signed_data(&self, seed: &[u8]) -> Vec<u8> {
        let mut signable = self.header.signable();
        signable.extend_from_slice(seed);
        self.data
            .write(&mut signable)
            .expect("Writing to vec should succeed");
        signable
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(any(feature = "faker", test), derive(fake::Dummy, Eq, PartialEq))]
#[allow(clippy::large_enum_variant)]
pub struct FaultData<V> {
    header: ConsensusHeader,
    sig: SignInfo,
    data: V,
}

impl<V: Serializable> Serializable for FaultData<V> {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        self.header.write(w)?;
        self.sig.write(w)?;
        self.data.write(w)?;
        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let header = ConsensusHeader::read(r)?;
        let sig = SignInfo::read(r)?;
        let data = V::read(r)?;

        Ok(Self { header, sig, data })
    }
}

impl Serializable for Fault {
    fn write<W: Write>(&self, w: &mut W) -> io::Result<()> {
        match self {
            Fault::DoubleCandidate(a, b) => {
                w.write_all(&[0u8])?;
                a.write(w)?;
                b.write(w)?;
            }
            Fault::DoubleRatificationVote(a, b) => {
                w.write_all(&[1u8])?;
                a.write(w)?;
                b.write(w)?;
            }
            Fault::DoubleValidationVote(a, b) => {
                w.write_all(&[2u8])?;
                a.write(w)?;
                b.write(w)?;
            }
        }

        Ok(())
    }

    fn read<R: Read>(r: &mut R) -> io::Result<Self>
    where
        Self: Sized,
    {
        let fault = Self::read_u8(r)?;
        let fault = match fault {
            0 => {
                Fault::DoubleCandidate(FaultData::read(r)?, FaultData::read(r)?)
            }
            1 => Fault::DoubleRatificationVote(
                FaultData::read(r)?,
                FaultData::read(r)?,
            ),
            2 => Fault::DoubleValidationVote(
                FaultData::read(r)?,
                FaultData::read(r)?,
            ),
            p => {
                println!("{p}");
                Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid faul"))?
            }
        };
        Ok(fault)
    }
}
