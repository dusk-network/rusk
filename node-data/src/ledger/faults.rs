// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{
    bls::PublicKey,
    message::{
        payload::{
            Candidate, Ratification, RatificationResult, Validation, Vote,
        },
        ConsensusHeader, SignInfo, SignedStepMessage,
    },
};

use dusk_bytes::Serializable as DuskSerializeble;
use execution_core::{
    signatures::bls::{
        Error as BlsSigError, MultisigPublicKey as BlsMultisigPublicKey,
        MultisigSignature as BlsMultisigSignature,
    },
    stake::EPOCH,
};
use thiserror::Error;
use tracing::error;

use super::*;

#[derive(Debug, Clone)]
#[cfg_attr(any(feature = "faker", test), derive(fake::Dummy, Eq, PartialEq))]
pub enum Fault {
    DoubleCandidate(FaultData<Hash>, FaultData<Hash>),
    DoubleRatificationVote(FaultData<Vote>, FaultData<Vote>),
    DoubleValidationVote(FaultData<Vote>, FaultData<Vote>),
}

impl Fault {
    pub fn size(&self) -> usize {
        // prev_block_hash + round + iter
        const FAULT_CONSENSUS_HEADER_SIZE: usize = 32 + u64::SIZE + u8::SIZE;
        // signer + signature
        const FAULT_SIG_INFO_SIZE: usize =
            BlsMultisigPublicKey::SIZE + BlsMultisigSignature::SIZE;

        const HEADERS: usize = FAULT_CONSENSUS_HEADER_SIZE * 2;
        const SIG_INFOS: usize = FAULT_SIG_INFO_SIZE * 2;
        let faults_data_size = match self {
            Fault::DoubleCandidate(..) => 32 * 2,
            Fault::DoubleRatificationVote(a, b) => {
                a.data.size() + b.data.size()
            }
            Fault::DoubleValidationVote(a, b) => a.data.size() + b.data.size(),
        };

        HEADERS + SIG_INFOS + faults_data_size
    }
}

#[derive(Debug, Error)]
pub enum InvalidFault {
    #[error("Inner faults have same data")]
    Duplicated,
    #[error("Fault is expired")]
    Expired,
    #[error("Fault is from future")]
    Future,
    #[error("Fault is from genesis block")]
    Genesis,
    #[error("Previous hash mismatch")]
    PrevHashMismatch,
    #[error("Iteration mismatch")]
    IterationMismatch,
    #[error("Faults related to emergency iteration")]
    EmergencyIteration,
    #[error("Round mismatch")]
    RoundMismatch,
    #[error("Invalid Signature {0}")]
    InvalidSignature(BlsSigError),
    #[error("Generic error {0}")]
    Other(String),
}

impl From<BlsSigError> for InvalidFault {
    fn from(value: BlsSigError) -> Self {
        Self::InvalidSignature(value)
    }
}

impl Fault {
    /// Hash the serialized form
    pub fn hash(&self) -> [u8; 32] {
        let mut b = vec![];
        self.write(&mut b).expect("Write to a vec shall not fail");
        sha3::Sha3_256::digest(&b[..]).into()
    }

    pub fn same(&self, other: &Fault) -> bool {
        let (a, b) = &self.faults_id();
        other.has_id(a) && other.has_id(b)
    }

    fn has_id(&self, id: &[u8; 32]) -> bool {
        let (a, b) = &self.faults_id();
        a == id || b == id
    }

    /// Return the IDs of the inner faults.
    fn faults_id(&self) -> (Hash, Hash) {
        match self {
            Fault::DoubleCandidate(a, b) => {
                let seed = Candidate::SIGN_SEED;
                let a = sha3::Sha3_256::digest(a.get_signed_data(seed)).into();
                let b = sha3::Sha3_256::digest(b.get_signed_data(seed)).into();
                (a, b)
            }
            Fault::DoubleRatificationVote(a, b) => {
                let seed = Ratification::SIGN_SEED;
                let a = sha3::Sha3_256::digest(a.get_signed_data(seed)).into();
                let b = sha3::Sha3_256::digest(b.get_signed_data(seed)).into();
                (a, b)
            }
            Fault::DoubleValidationVote(a, b) => {
                let seed = Validation::SIGN_SEED;
                let a = sha3::Sha3_256::digest(a.get_signed_data(seed)).into();
                let b = sha3::Sha3_256::digest(b.get_signed_data(seed)).into();
                (a, b)
            }
        }
    }

    fn to_culprit(&self) -> PublicKey {
        match self {
            Fault::DoubleRatificationVote(a, _)
            | Fault::DoubleValidationVote(a, _) => a.sig.signer.clone(),
            Fault::DoubleCandidate(a, _) => a.sig.signer.clone(),
        }
    }

    /// Get the ConsensusHeader related to the inner FaultDatas
    fn consensus_header(&self) -> (&ConsensusHeader, &ConsensusHeader) {
        match self {
            Fault::DoubleRatificationVote(a, b)
            | Fault::DoubleValidationVote(a, b) => (&a.header, &b.header),
            Fault::DoubleCandidate(a, b) => (&a.header, &b.header),
        }
    }

    /// Check if both faults are related to the same consensus header and
    /// validate their signatures.
    ///
    /// Return the related ConsensusHeader
    pub fn validate(
        &self,
        current_height: u64,
    ) -> Result<&ConsensusHeader, InvalidFault> {
        let (h1, h2) = self.consensus_header();
        // Check that both consensus headers are the same
        if h1.iteration != h2.iteration {
            return Err(InvalidFault::IterationMismatch);
        }
        if h1.round != h2.round {
            return Err(InvalidFault::RoundMismatch);
        }
        if h1.prev_block_hash != h2.prev_block_hash {
            return Err(InvalidFault::PrevHashMismatch);
        }

        // Check that fault refers to different fault_data
        let (id_a, id_b) = self.faults_id();
        if id_a == id_b {
            return Err(InvalidFault::Duplicated);
        }

        if h1.round == 0 {
            return Err(InvalidFault::Genesis);
        }

        // Check that fault is not expired. A fault expires after an epoch
        if h1.round < current_height.saturating_sub(EPOCH) {
            return Err(InvalidFault::Expired);
        }

        // Check that fault is related to something that is already processed
        if h1.round > current_height {
            return Err(InvalidFault::Future);
        }

        // Verify signatures
        self.verify_sigs()?;

        Ok(h1)
    }

    /// Check if both faults are signed properly
    fn verify_sigs(&self) -> Result<(), BlsSigError> {
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
        let sig = BlsMultisigSignature::from_bytes(signature)?;
        let pk = BlsMultisigPublicKey::aggregate(&[*sign_info.signer.inner()])?;
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
            p => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid fault: {p}"),
            ))?,
        };
        Ok(fault)
    }
}

#[derive(Clone, Debug)]
pub struct Slash {
    pub provisioner: PublicKey,
    pub r#type: SlashType,
}

#[derive(Clone, Debug)]
pub enum SlashType {
    Soft,
    Hard,
    HardWithSeverity(u8),
}

impl Slash {
    fn from_iteration_info(
        value: &IterationInfo,
    ) -> Result<Option<Self>, dusk_bytes::Error> {
        let (attestation, provisioner) = value;
        let slash = match attestation.result {
            RatificationResult::Fail(Vote::NoCandidate) => SlashType::Soft,
            RatificationResult::Fail(Vote::Invalid(_)) => SlashType::Hard,
            _ => {
                return Ok(None);
            }
        };
        let provisioner = (*provisioner.inner()).try_into().map_err(|e| {
            error!("Unable to generate provisioners from IterationInfo: {e:?}");
            e
        })?;
        Ok(Some(Self {
            provisioner,
            r#type: slash,
        }))
    }

    pub fn from_block(block: &Block) -> Result<Vec<Slash>, io::Error> {
        Self::from_iterations_and_faults(
            &block.header().failed_iterations,
            block.faults(),
        )
    }

    pub fn from_iterations_and_faults(
        failed_iterations: &IterationsInfo,
        faults: &[Fault],
    ) -> Result<Vec<Slash>, io::Error> {
        let mut slashing = failed_iterations
            .att_list
            .iter()
            .flatten()
            .flat_map(Slash::from_iteration_info)
            .flatten()
            .collect::<Vec<_>>();
        slashing.extend(faults.iter().map(Slash::from));
        Ok(slashing)
    }
}

impl From<&Fault> for Slash {
    fn from(value: &Fault) -> Self {
        let slash_type = match value {
            Fault::DoubleCandidate(_, _)
            | Fault::DoubleRatificationVote(_, _)
            | Fault::DoubleValidationVote(_, _) => {
                SlashType::HardWithSeverity(2u8)
            }
        };
        let provisioner = value.to_culprit();
        Self {
            provisioner,
            r#type: slash_type,
        }
    }
}
