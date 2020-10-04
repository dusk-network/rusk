// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::services::rusk_proto;
use crate::transaction::{Transaction, TransactionPayload};
use core::convert::{TryFrom, TryInto};
use dusk_pki::{
    Ownable, PublicSpendKey, SecretSpendKey, StealthAddress, ViewKey,
};
use dusk_plonk::bls12_381::Scalar as BlsScalar;
use dusk_plonk::jubjub::{
    AffinePoint as JubJubAffine, ExtendedPoint as JubJubExtended,
    Scalar as JubJubScalar,
};
use dusk_plonk::proof_system::Proof;
use phoenix_core::{Crossover, Fee, Note};
use poseidon252::cipher::{PoseidonCipher, CIPHER_BYTES_SIZE};
use std::io::{Read, Write};
use tonic::{Code, Status};

/// Generic function used to retrieve parameters that are optional from a
/// GRPC request.
pub(crate) fn decode_request_param<T, U>(
    possible_param: Option<&T>,
) -> Result<U, Status>
where
    T: Clone,
    U: TryFrom<T, Error = Status>,
{
    Ok(U::try_from(
        possible_param
            .ok_or(Status::new(Code::Unknown, "Missing required fields."))?
            .clone(),
    )?)
}

/// Generic function used to encore parameters that are optional in a
/// GRPC response.
pub(crate) fn encode_request_param<T, U>(param: T) -> Option<U>
where
    U: From<T>,
{
    Some(U::from(param))
}

// ---- Basic Types -> Protobuf types ---- //
impl From<JubJubAffine> for rusk_proto::JubJubCompressed {
    fn from(value: JubJubAffine) -> Self {
        rusk_proto::JubJubCompressed {
            data: Vec::from(&value.to_bytes()[..]),
        }
    }
}

impl From<JubJubExtended> for rusk_proto::JubJubCompressed {
    fn from(value: JubJubExtended) -> Self {
        JubJubAffine::from(value).into()
    }
}

impl From<&JubJubExtended> for rusk_proto::JubJubCompressed {
    fn from(value: &JubJubExtended) -> Self {
        (*value).into()
    }
}

impl From<&JubJubScalar> for rusk_proto::JubJubScalar {
    fn from(value: &JubJubScalar) -> Self {
        rusk_proto::JubJubScalar {
            data: Vec::from(&value.to_bytes()[..]),
        }
    }
}

impl From<JubJubScalar> for rusk_proto::JubJubScalar {
    fn from(value: JubJubScalar) -> Self {
        (&value).into()
    }
}

impl From<&BlsScalar> for rusk_proto::BlsScalar {
    fn from(value: &BlsScalar) -> Self {
        rusk_proto::BlsScalar {
            data: Vec::from(&value.to_bytes()[..]),
        }
    }
}

impl From<BlsScalar> for rusk_proto::BlsScalar {
    fn from(value: BlsScalar) -> Self {
        (&value).into()
    }
}

impl From<PublicSpendKey> for rusk_proto::PublicKey {
    fn from(value: PublicSpendKey) -> Self {
        rusk_proto::PublicKey {
            a_g: Some(JubJubAffine::from(value.A()).into()),
            b_g: Some(JubJubAffine::from(value.B()).into()),
        }
    }
}

impl From<SecretSpendKey> for rusk_proto::SecretKey {
    fn from(value: SecretSpendKey) -> Self {
        rusk_proto::SecretKey {
            a: Some(value.a().into()),
            b: Some(value.b().into()),
        }
    }
}

impl From<ViewKey> for rusk_proto::ViewKey {
    fn from(value: ViewKey) -> Self {
        rusk_proto::ViewKey {
            a: Some(value.a().into()),
            b_g: Some(JubJubAffine::from(value.B()).into()),
        }
    }
}

impl From<StealthAddress> for rusk_proto::StealthAddress {
    fn from(value: StealthAddress) -> Self {
        rusk_proto::StealthAddress {
            r_g: Some(JubJubAffine::from(value.R()).into()),
            pk_r: Some(JubJubAffine::from(value.pk_r()).into()),
        }
    }
}

impl From<&StealthAddress> for rusk_proto::StealthAddress {
    fn from(value: &StealthAddress) -> Self {
        (*value).into()
    }
}

impl From<PoseidonCipher> for rusk_proto::PoseidonCipher {
    fn from(value: PoseidonCipher) -> Self {
        rusk_proto::PoseidonCipher {
            data: value.to_bytes().to_vec(),
        }
    }
}

impl From<&Proof> for rusk_proto::Proof {
    fn from(value: &Proof) -> Self {
        rusk_proto::Proof {
            data: value.to_bytes().to_vec(),
        }
    }
}

impl TryFrom<&mut Note> for rusk_proto::Note {
    type Error = Status;

    fn try_from(value: &mut Note) -> Result<Self, Status> {
        let mut bytes = [0u8; 233];
        value.read(&mut bytes)?;
        Ok(rusk_proto::Note {
            note_type: value.note() as i32,
            value_commitment: Some(value.value_commitment().into()),
            nonce: Some(value.nonce().into()),
            address: Some(value.stealth_address().into()),
            pos: value.pos(),
            encrypted_data: Some(rusk_proto::PoseidonCipher {
                data: bytes[137..233].to_vec(),
            }),
        })
    }
}

impl From<Fee> for rusk_proto::Fee {
    fn from(value: Fee) -> Self {
        rusk_proto::Fee {
            gas_limit: value.gas_limit,
            gas_price: value.gas_price,
            address: Some(value.stealth_address().into()),
        }
    }
}

impl From<Crossover> for rusk_proto::Crossover {
    fn from(value: Crossover) -> Self {
        rusk_proto::Crossover {
            value_comm: Some(
                JubJubAffine::from(value.value_commitment()).into(),
            ),
            nonce: Some((*value.nonce()).into()),
            /// XXX: fix this typo in rusk-schema
            encypted_data: Some((*value.encrypted_data()).into()),
        }
    }
}

impl TryFrom<&TransactionPayload> for rusk_proto::TransactionPayload {
    type Error = Status;

    fn try_from(value: &TransactionPayload) -> Result<Self, Status> {
        Ok(rusk_proto::TransactionPayload {
            anchor: Some(value.anchor().into()),
            nullifier: value
                .nullifiers()
                .to_vec()
                .into_iter()
                .map(|n| n.into())
                .collect(),
            crossover: value.crossover().map(|c| c.into()),
            notes: value
                .notes()
                .to_vec()
                .into_iter()
                .map(|mut n| (&mut n).try_into())
                .collect::<Result<Vec<rusk_proto::Note>, _>>()?,
            fee: Some(value.fee().into()),
            spending_proof: Some(value.spending_proof().into()),
            call_data: value.call_data().to_vec(),
        })
    }
}

impl TryFrom<TransactionPayload> for rusk_proto::TransactionPayload {
    type Error = Status;

    fn try_from(value: TransactionPayload) -> Result<Self, Status> {
        (&value).try_into()
    }
}

impl TryFrom<Transaction> for rusk_proto::Transaction {
    type Error = Status;

    fn try_from(value: Transaction) -> Result<Self, Status> {
        Ok(rusk_proto::Transaction {
            version: value.version().into(),
            r#type: value.tx_type().into(),
            tx_payload: Some(value.payload().try_into()?),
        })
    }
}

// ----- Protobuf types -> Basic types ----- //
impl TryFrom<&rusk_proto::BlsScalar> for BlsScalar {
    type Error = Status;

    fn try_from(value: &rusk_proto::BlsScalar) -> Result<BlsScalar, Status> {
        let mut bytes = [0u8; 32];
        // Check if the data is 32 bytes exactly so we can
        // safely copy from the slice.
        if value.data.len() != 32 {
            return Err(Status::failed_precondition(
                "BlsScalar recieved is not 32 bytes long",
            ));
        };
        bytes[..].copy_from_slice(&value.data[..]);
        Option::from(BlsScalar::from_bytes(&bytes)).ok_or(
            Status::failed_precondition(
                "BlsScalar was not cannonically encoded",
            ),
        )
    }
}

impl TryFrom<rusk_proto::BlsScalar> for BlsScalar {
    type Error = Status;

    fn try_from(value: rusk_proto::BlsScalar) -> Result<BlsScalar, Status> {
        (&value).try_into()
    }
}

impl TryFrom<&rusk_proto::JubJubScalar> for JubJubScalar {
    type Error = Status;

    fn try_from(
        value: &rusk_proto::JubJubScalar,
    ) -> Result<JubJubScalar, Status> {
        let mut bytes = [0u8; 32];
        // Check if the data is 32 bytes exactly so we can
        // safely copy from the slice.
        if value.data.len() != 32 {
            return Err(Status::failed_precondition(
                "JubJubScalar recieved is not 32 bytes long",
            ));
        };
        bytes[..].copy_from_slice(&value.data[..]);
        Option::from(JubJubScalar::from_bytes(&bytes)).ok_or(
            Status::failed_precondition(
                "JubJubScalar was not cannonically encoded",
            ),
        )
    }
}

impl TryFrom<&rusk_proto::JubJubCompressed> for JubJubAffine {
    type Error = Status;

    fn try_from(
        value: &rusk_proto::JubJubCompressed,
    ) -> Result<JubJubAffine, Status> {
        let mut bytes = [0u8; 32];
        // Check if the data is 32 bytes exactly so we can
        // safely copy from the slice.
        if value.data.len() != 32 {
            return Err(Status::failed_precondition(
                "JubJubAffine recieved is not 32 bytes long",
            ));
        };
        bytes[..].copy_from_slice(&value.data[..]);
        Option::from(JubJubAffine::from_bytes(bytes)).ok_or(
            Status::failed_precondition(
                "JubJubAffine was not cannonically encoded",
            ),
        )
    }
}

impl TryFrom<&rusk_proto::JubJubCompressed> for JubJubExtended {
    type Error = Status;

    fn try_from(
        value: &rusk_proto::JubJubCompressed,
    ) -> Result<JubJubExtended, Status> {
        let a: JubJubAffine = value.try_into()?;
        Ok(a.into())
    }
}

impl TryFrom<&rusk_proto::PublicKey> for PublicSpendKey {
    type Error = Status;

    fn try_from(
        value: &rusk_proto::PublicKey,
    ) -> Result<PublicSpendKey, Status> {
        Ok(
            PublicSpendKey::new(
                decode_request_param::<
                    &rusk_proto::JubJubCompressed,
                    JubJubAffine,
                >(value.a_g.as_ref().as_ref())?
                .into(),
                decode_request_param::<
                    &rusk_proto::JubJubCompressed,
                    JubJubAffine,
                >(value.a_g.as_ref().as_ref())?
                .into(),
            ),
        )
    }
}

impl TryFrom<&rusk_proto::StealthAddress> for StealthAddress {
    type Error = Status;

    fn try_from(
        value: &rusk_proto::StealthAddress,
    ) -> Result<StealthAddress, Status> {
        // Ensure that both fields are not empty
        let r_g = value.r_g.as_ref().ok_or(Status::failed_precondition(
            "StealthAddress was missing r_g field",
        ))?;
        let pk_r = value.pk_r.as_ref().ok_or(Status::failed_precondition(
            "StealthAddress was missing pk_r field",
        ))?;

        // Ensure that both fields are 32 bytes long, so we can
        // safely copy from the slice.
        if r_g.data.len() != 32 || pk_r.data.len() != 32 {
            return Err(Status::failed_precondition(
                "StealthAddress fields are of improper length",
            ));
        };

        let mut bytes = [0u8; 64];
        bytes[..32].copy_from_slice(&r_g.data[..]);
        bytes[32..].copy_from_slice(&pk_r.data[..]);

        Ok(StealthAddress::from_bytes(&bytes).map_err(|_| {
            Status::failed_precondition("StealthAdress was improperly encoded")
        })?)
    }
}

impl TryFrom<&rusk_proto::PoseidonCipher> for PoseidonCipher {
    type Error = Status;

    fn try_from(
        value: &rusk_proto::PoseidonCipher,
    ) -> Result<PoseidonCipher, Status> {
        // Ensure that data field is proper length
        if value.data.len() != CIPHER_BYTES_SIZE {
            return Err(Status::failed_precondition(
                "PoseidonCipher data field is of improper length",
            ));
        }

        let mut bytes = [0u8; CIPHER_BYTES_SIZE];
        bytes.copy_from_slice(&value.data);

        Ok(PoseidonCipher::from_bytes(&bytes).ok_or(
            Status::failed_precondition("Could not decode PoseidonCipher"),
        )?)
    }
}

impl TryFrom<&rusk_proto::Proof> for Proof {
    type Error = Status;

    fn try_from(value: &rusk_proto::Proof) -> Result<Proof, Status> {
        Ok(Proof::from_bytes(&value.data).map_err(|_| {
            Status::failed_precondition("Could not decode proof")
        })?)
    }
}

impl TryFrom<&rusk_proto::Note> for Note {
    type Error = Status;

    fn try_from(value: &rusk_proto::Note) -> Result<Note, Status> {
        let mut buf = [0u8; 233];
        let t = value.note_type as u8;
        buf[0] = t;
        buf[1..33].copy_from_slice(
            &value
                .value_commitment
                .as_ref()
                .ok_or(Status::failed_precondition(
                    "No value commitment present",
                ))?
                .data,
        );
        buf[33..65].copy_from_slice(
            &value
                .nonce
                .as_ref()
                .ok_or(Status::failed_precondition("No nonce present"))?
                .data,
        );
        buf[65..97].copy_from_slice(
            &value
                .address
                .as_ref()
                .ok_or(Status::failed_precondition("No nonce present"))?
                .r_g
                .as_ref()
                .ok_or(Status::failed_precondition(
                    "No pk_r present in stealth address",
                ))?
                .data,
        );
        buf[97..129].copy_from_slice(
            &value
                .address
                .as_ref()
                .ok_or(Status::failed_precondition("No nonce present"))?
                .pk_r
                .as_ref()
                .ok_or(Status::failed_precondition(
                    "No pk_r present in stealth address",
                ))?
                .data,
        );
        buf[129..137].copy_from_slice(&value.pos.to_le_bytes());
        buf[137..233].copy_from_slice(
            &value
                .encrypted_data
                .as_ref()
                .ok_or(Status::failed_precondition(
                    "No encrypted data present",
                ))?
                .data,
        );

        let mut note = Note::default();
        note.write(&mut buf)?;
        Ok(note)
    }
}

impl TryFrom<&rusk_proto::Fee> for Fee {
    type Error = Status;

    fn try_from(value: &rusk_proto::Fee) -> Result<Fee, Status> {
        let mut buf = [0u8; 80];
        buf[0..8].copy_from_slice(&value.gas_limit.to_le_bytes());
        buf[8..16].copy_from_slice(&value.gas_price.to_le_bytes());
        buf[16..48].copy_from_slice(
            &value
                .address
                .as_ref()
                .ok_or(Status::failed_precondition("No address present"))?
                .r_g
                .as_ref()
                .ok_or(Status::failed_precondition("No r_g present"))?
                .data,
        );
        buf[48..].copy_from_slice(
            &value
                .address
                .as_ref()
                .ok_or(Status::failed_precondition("No address present"))?
                .pk_r
                .as_ref()
                .ok_or(Status::failed_precondition("No pk_r present"))?
                .data,
        );
        let mut fee = Fee::default();
        fee.write(&mut buf)?;
        Ok(fee)
    }
}

impl TryFrom<&rusk_proto::Crossover> for Crossover {
    type Error = Status;

    fn try_from(value: &rusk_proto::Crossover) -> Result<Crossover, Status> {
        let mut buf = [0u8; 160];
        buf[0..32].copy_from_slice(
            &value
                .value_comm
                .as_ref()
                .ok_or(Status::failed_precondition(
                    "No nonce present in crossover",
                ))?
                .data,
        );
        buf[32..64].copy_from_slice(
            &value
                .nonce
                .as_ref()
                .ok_or(Status::failed_precondition(
                    "No nonce present in crossover",
                ))?
                .data,
        );
        buf[64..].copy_from_slice(
            &value
                .encypted_data
                .as_ref()
                .ok_or(Status::failed_precondition(
                    "No encrypted data present in crossover",
                ))?
                .data,
        );
        let mut crossover = Crossover::default();
        crossover.write(&mut buf)?;
        Ok(crossover)
    }
}

impl TryFrom<&rusk_proto::TransactionPayload> for TransactionPayload {
    type Error = Status;

    fn try_from(
        value: &rusk_proto::TransactionPayload,
    ) -> Result<TransactionPayload, Status> {
        let mut crossover: Option<Crossover> = None;
        if value.crossover.as_ref().is_some() {
            crossover = Some(value.crossover.as_ref().unwrap().try_into()?);
        }

        Ok(TransactionPayload::new(
            value
                .anchor
                .as_ref()
                .ok_or(Status::failed_precondition(
                    "No anchor present in transaction payload",
                ))?
                .try_into()?,
            value
                .nullifier
                .iter()
                .map(|n| n.try_into())
                .collect::<Result<Vec<BlsScalar>, _>>()?,
            crossover,
            value
                .notes
                .iter()
                .map(|n| n.try_into())
                .collect::<Result<Vec<Note>, _>>()?,
            value
                .fee
                .as_ref()
                .ok_or(Status::failed_precondition(
                    "No fee present in transaction payload",
                ))?
                .try_into()?,
            value
                .spending_proof
                .as_ref()
                .ok_or(Status::failed_precondition(
                    "No proof present in transaction payload",
                ))?
                .try_into()?,
            value.call_data.clone(),
        ))
    }
}

impl TryFrom<rusk_proto::TransactionPayload> for TransactionPayload {
    type Error = Status;

    fn try_from(
        value: rusk_proto::TransactionPayload,
    ) -> Result<TransactionPayload, Status> {
        (&value).try_into()
    }
}

impl TryFrom<&rusk_proto::Transaction> for Transaction {
    type Error = Status;

    fn try_from(
        value: &rusk_proto::Transaction,
    ) -> Result<Transaction, Status> {
        Ok(Transaction::new(
            value
                .version
                .try_into()
                .map_err(|e| Status::failed_precondition(format!("{}", e)))?,
            value
                .r#type
                .try_into()
                .map_err(|e| Status::failed_precondition(format!("{}", e)))?,
            value
                .tx_payload
                .as_ref()
                .ok_or(Status::failed_precondition(
                    "No transaction payload present",
                ))?
                .try_into()?,
        ))
    }
}
