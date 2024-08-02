// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::vec;
use alloc::vec::Vec;

use dusk_bytes::{
    DeserializableSlice, Error as BytesError, Serializable, Write,
};
use execution_core::{
    plonk::Proof,
    signatures::schnorr::{
        Signature as SchnorrSignature,
        SignatureDouble as SchnorrSignatureDouble,
    },
    transfer::phoenix::{
        Note, Payload as PhoenixPayload, PublicKey as PhoenixPublicKey,
        SecretKey as PhoenixSecretKey, Transaction as PhoenixTransaction,
        NOTES_TREE_DEPTH, OUTPUT_NOTES,
    },
    BlsScalar, JubJubAffine, JubJubExtended, JubJubScalar,
    GENERATOR_NUMS_EXTENDED,
};

use poseidon_merkle::Opening as PoseidonOpening;
use rand_core::{CryptoRng, RngCore};

/// An input to a transaction that is yet to be proven.
#[derive(PartialEq, Debug, Clone)]
pub struct UnprovenTransactionInput {
    pub nullifier: BlsScalar,
    pub opening: PoseidonOpening<(), NOTES_TREE_DEPTH>,
    pub note: Note,
    pub value: u64,
    pub value_blinder: JubJubScalar,
    pub npk_prime: JubJubExtended,
    pub sig: SchnorrSignatureDouble,
}

impl UnprovenTransactionInput {
    pub fn new<Rng: RngCore + CryptoRng>(
        rng: &mut Rng,
        sender_sk: &PhoenixSecretKey,
        note: Note,
        value: u64,
        value_blinder: JubJubScalar,
        opening: PoseidonOpening<(), NOTES_TREE_DEPTH>,
        payload_hash: BlsScalar,
    ) -> Self {
        let nullifier = note.gen_nullifier(sender_sk);
        let nsk = sender_sk.gen_note_sk(note.stealth_address());
        let sig = nsk.sign_double(rng, payload_hash);

        let npk_prime = GENERATOR_NUMS_EXTENDED * nsk.as_ref();

        Self {
            note,
            value,
            value_blinder,
            sig,
            nullifier,
            opening,
            npk_prime,
        }
    }

    /// Serialize the input to a variable size byte buffer.
    pub fn to_var_bytes(&self) -> Vec<u8> {
        let affine_npk_p = JubJubAffine::from(&self.npk_prime);

        let opening_bytes = rkyv::to_bytes::<_, 256>(&self.opening)
            .expect("Rkyv serialization should always succeed for an opening")
            .to_vec();

        let mut bytes = Vec::with_capacity(
            BlsScalar::SIZE
                + Note::SIZE
                + JubJubAffine::SIZE
                + SchnorrSignatureDouble::SIZE
                + u64::SIZE
                + JubJubScalar::SIZE
                + opening_bytes.len(),
        );

        bytes.extend_from_slice(&self.nullifier.to_bytes());
        bytes.extend_from_slice(&self.note.to_bytes());
        bytes.extend_from_slice(&self.value.to_bytes());
        bytes.extend_from_slice(&self.value_blinder.to_bytes());
        bytes.extend_from_slice(&affine_npk_p.to_bytes());
        bytes.extend_from_slice(&self.sig.to_bytes());
        bytes.extend(opening_bytes);

        bytes
    }

    /// Deserializes the the input from bytes.
    pub fn from_slice(buf: &[u8]) -> Result<Self, BytesError> {
        let mut bytes = buf;

        let nullifier = BlsScalar::from_reader(&mut bytes)?;
        let note = Note::from_reader(&mut bytes)?;
        let value = u64::from_reader(&mut bytes)?;
        let value_blinder = JubJubScalar::from_reader(&mut bytes)?;
        let npk_prime =
            JubJubExtended::from(JubJubAffine::from_reader(&mut bytes)?);
        let sig = SchnorrSignatureDouble::from_reader(&mut bytes)?;

        // `to_vec` is required here otherwise `rkyv` will throw an alignment
        // error
        #[allow(clippy::unnecessary_to_owned)]
        let opening = rkyv::from_bytes(&bytes.to_vec())
            .map_err(|_| BytesError::InvalidData)?;

        Ok(Self {
            note,
            value,
            value_blinder,
            sig,
            nullifier,
            opening,
            npk_prime,
        })
    }

    /// Returns the nullifier of the input.
    pub fn nullifier(&self) -> BlsScalar {
        self.nullifier
    }

    /// Returns the opening of the input.
    pub fn opening(&self) -> &PoseidonOpening<(), NOTES_TREE_DEPTH> {
        &self.opening
    }

    /// Returns the note of the input.
    pub fn note(&self) -> &Note {
        &self.note
    }

    /// Returns the value of the input.
    pub fn value(&self) -> u64 {
        self.value
    }

    /// Returns the blinding factor for the value of the input.
    pub fn value_blinder(&self) -> JubJubScalar {
        self.value_blinder
    }

    /// Returns the input's note public key prime.
    pub fn note_pk_prime(&self) -> JubJubExtended {
        self.npk_prime
    }

    /// Returns the input's signature.
    pub fn signature(&self) -> &SchnorrSignatureDouble {
        &self.sig
    }
}

/// A transaction that is yet to be proven. The purpose of this is solely to
/// send to the node to perform a circuit proof.
#[derive(PartialEq, Debug, Clone)]
pub struct UnprovenTransaction {
    pub inputs: Vec<UnprovenTransactionInput>,
    pub outputs: [(Note, u64, JubJubScalar, [JubJubScalar; 2]); OUTPUT_NOTES],
    pub payload: PhoenixPayload,
    pub sender_pk: PhoenixPublicKey,
    pub signatures: (SchnorrSignature, SchnorrSignature),
}

impl UnprovenTransaction {
    /// Consumes self and a proof to generate a transaction.
    pub fn gen_transaction(self, proof: Proof) -> PhoenixTransaction {
        PhoenixTransaction::new(self.payload, proof.to_bytes())
    }

    /// Serialize the transaction to a variable length byte buffer.
    #[allow(unused_must_use)]
    pub fn to_var_bytes(&self) -> Vec<u8> {
        let serialized_inputs: Vec<Vec<u8>> = self
            .inputs
            .iter()
            .map(UnprovenTransactionInput::to_var_bytes)
            .collect();
        let num_inputs = self.inputs.len();
        let total_input_len = serialized_inputs
            .iter()
            .fold(0, |len, input| len + input.len());

        const OUTPUT_SIZE: usize = Note::SIZE
            + u64::SIZE
            + JubJubScalar::SIZE
            + 2 * JubJubScalar::SIZE;
        let serialized_outputs: Vec<[u8; OUTPUT_SIZE]> = self
            .outputs
            .iter()
            .map(|(note, value, value_blinder, sender_blinder)| {
                let mut buf = [0; OUTPUT_SIZE];

                buf[..Note::SIZE].copy_from_slice(&note.to_bytes());
                buf[Note::SIZE..Note::SIZE + u64::SIZE]
                    .copy_from_slice(&value.to_bytes());
                buf[Note::SIZE + u64::SIZE
                    ..Note::SIZE + u64::SIZE + JubJubScalar::SIZE]
                    .copy_from_slice(&value_blinder.to_bytes());
                let mut start = Note::SIZE + u64::SIZE + JubJubScalar::SIZE;
                buf[start..start + JubJubScalar::SIZE]
                    .copy_from_slice(&sender_blinder[0].to_bytes());
                start += JubJubScalar::SIZE;
                buf[start..start + JubJubScalar::SIZE]
                    .copy_from_slice(&sender_blinder[1].to_bytes());

                buf
            })
            .collect();
        let num_outputs = self.outputs.len();
        let total_output_len = serialized_outputs
            .iter()
            .fold(0, |len, output| len + output.len());

        let payload_bytes = self.payload.to_var_bytes();

        let size =
            // the amount of inputs
            u64::SIZE
            // the len of each input item
            + num_inputs * u64::SIZE
            // the total amount of bytes of the inputs
            + total_input_len
            // the amount of outputs
            + u64::SIZE
            // the total amount of bytes of the outputs
            + total_output_len
            // the total amount of bytes of the payload
            + u64::SIZE
            // the payload
            + payload_bytes.len()
            // the payload-hash
            + BlsScalar::SIZE
            // the sender-pk
            + PhoenixPublicKey::SIZE
            // the two signatures
            + 2 * SchnorrSignature::SIZE;

        let mut buf = vec![0; size];
        let mut writer = &mut buf[..];

        writer.write(&(num_inputs as u64).to_bytes());
        for sinput in serialized_inputs {
            writer.write(&(sinput.len() as u64).to_bytes());
            writer.write(&sinput);
        }

        writer.write(&(num_outputs as u64).to_bytes());
        for soutput in serialized_outputs {
            writer.write(&soutput);
        }

        writer.write(&(payload_bytes.len() as u64).to_bytes());
        writer.write(&payload_bytes[..]);

        writer.write(&self.sender_pk.to_bytes());
        writer.write(&self.signatures.0.to_bytes());
        writer.write(&self.signatures.1.to_bytes());

        buf
    }

    /// Deserialize the transaction from a bytes buffer.
    pub fn from_slice(buf: &[u8]) -> Result<Self, BytesError> {
        let mut buffer = buf;

        let num_inputs = u64::from_reader(&mut buffer)?;
        let mut inputs = Vec::with_capacity(num_inputs as usize);
        for _ in 0..num_inputs {
            let size = u64::from_reader(&mut buffer)? as usize;
            inputs.push(UnprovenTransactionInput::from_slice(&buffer[..size])?);
            buffer = &buffer[size..];
        }

        let num_outputs = u64::from_reader(&mut buffer)?;
        let mut outputs = Vec::with_capacity(num_outputs as usize);
        for _ in 0..num_outputs {
            let note = Note::from_reader(&mut buffer)?;
            let value = u64::from_reader(&mut buffer)?;
            let value_blinder = JubJubScalar::from_reader(&mut buffer)?;
            let sender_blinder_a = JubJubScalar::from_reader(&mut buffer)?;
            let sender_blinder_b = JubJubScalar::from_reader(&mut buffer)?;

            outputs.push((
                note,
                value,
                value_blinder,
                [sender_blinder_a, sender_blinder_b],
            ));
        }
        let outputs: [(Note, u64, JubJubScalar, [JubJubScalar; 2]);
            OUTPUT_NOTES] =
            outputs.try_into().map_err(|_| BytesError::InvalidData)?;

        let payload_len = u64::from_reader(&mut buffer)?;
        let payload = PhoenixPayload::from_slice(buffer)?;
        let mut buffer = &buffer[payload_len as usize..];

        let sender_pk = PhoenixPublicKey::from_reader(&mut buffer)?;
        let sig_a = SchnorrSignature::from_reader(&mut buffer)?;
        let sig_b = SchnorrSignature::from_reader(&mut buffer)?;

        Ok(Self {
            inputs,
            outputs,
            payload,
            sender_pk,
            signatures: (sig_a, sig_b),
        })
    }

    /// Returns the inputs to the transaction.
    pub fn inputs(&self) -> &[UnprovenTransactionInput] {
        &self.inputs
    }

    /// Returns the outputs of the transaction.
    pub fn outputs(&self) -> &[(Note, u64, JubJubScalar, [JubJubScalar; 2])] {
        &self.outputs
    }

    /// Returns the payload of the transaction.
    pub fn payload(&self) -> &PhoenixPayload {
        &self.payload
    }

    /// Returns the payload-hash of the transaction.
    pub fn payload_hash(&self) -> BlsScalar {
        self.payload.hash()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use execution_core::{
        signatures::schnorr::SecretKey as SchnorrSecretKey,
        transfer::{
            contract_exec::{ContractCall, ContractExec},
            phoenix::{Fee, TxSkeleton},
        },
    };
    use poseidon_merkle::{Item, Tree};
    use rand::{rngs::StdRng, SeedableRng};

    #[test]
    fn serialize_deserialize() -> Result<(), BytesError> {
        let mut rng = StdRng::seed_from_u64(0xbeef);
        let sender_sk = PhoenixSecretKey::random(&mut rng);
        let sender_pk = PhoenixPublicKey::from(&sender_sk);
        let receiver_pk =
            PhoenixPublicKey::from(&PhoenixSecretKey::random(&mut rng));

        let transfer_value = 42;
        let transfer_value_blinder = JubJubScalar::from(5647890216u64);
        let transfer_sender_blinder =
            [JubJubScalar::from(57u64), JubJubScalar::from(789u64)];
        let transfer_note = Note::obfuscated(
            &mut rng,
            &sender_pk,
            &receiver_pk,
            transfer_value,
            transfer_value_blinder,
            transfer_sender_blinder,
        );
        let change_value = 24;
        let change_sender_blinder =
            [JubJubScalar::from(7483u64), JubJubScalar::from(265829u64)];
        let change_note = Note::transparent(
            &mut rng,
            &sender_pk,
            &receiver_pk,
            change_value,
            transfer_sender_blinder,
        );
        let tx_skeleton = TxSkeleton {
            root: BlsScalar::from(1),
            nullifiers: vec![
                BlsScalar::from(2),
                BlsScalar::from(3),
                BlsScalar::from(4),
                BlsScalar::from(5),
            ],
            outputs: [transfer_note.clone(), change_note.clone()],
            max_fee: 10000,
            deposit: 20,
        };

        let fee = Fee::new(&mut rng, &sender_pk, 4242, 42);
        let call = ContractCall::new(
            [10; 32],
            "some method",
            &vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        )
        .unwrap();

        let payload = PhoenixPayload {
            tx_skeleton,
            fee,
            exec: Some(ContractExec::Call(call)),
        };

        let sender_blinder_1 =
            [JubJubScalar::from(521u64), JubJubScalar::from(6521u64)];
        let sender_blinder_2 = [
            JubJubScalar::from(585631u64),
            JubJubScalar::from(65658151u64),
        ];
        let value1 = 100;
        let value2 = 200;
        let note1 = Note::transparent(
            &mut rng,
            &sender_pk,
            &receiver_pk,
            value1,
            sender_blinder_1,
        );
        let note2 = Note::transparent(
            &mut rng,
            &sender_pk,
            &receiver_pk,
            value2,
            sender_blinder_2,
        );
        let mut tree = Tree::new();
        let pos1 = 12;
        tree.insert(
            pos1,
            Item {
                hash: note1.hash(),
                data: (),
            },
        );
        let pos2 = 13;
        tree.insert(
            pos2,
            Item {
                hash: note2.hash(),
                data: (),
            },
        );
        let opening1 = tree.opening(pos1).unwrap();
        let opening2 = tree.opening(pos2).unwrap();

        let payload_hash = payload.hash();
        let inputs = vec![
            UnprovenTransactionInput::new(
                &mut rng,
                &sender_sk,
                note1,
                value1,
                JubJubScalar::zero(),
                opening1,
                payload_hash,
            ),
            UnprovenTransactionInput::new(
                &mut rng,
                &sender_sk,
                note2,
                value2,
                JubJubScalar::zero(),
                opening2,
                payload_hash,
            ),
        ];

        let schnorr_sk_a = SchnorrSecretKey::from(sender_sk.a());
        let sig_a = schnorr_sk_a.sign(&mut rng, payload_hash);
        let schnorr_sk_b = SchnorrSecretKey::from(sender_sk.b());
        let sig_b = schnorr_sk_b.sign(&mut rng, payload_hash);

        let utx = UnprovenTransaction {
            inputs,
            outputs: [
                (
                    transfer_note,
                    transfer_value,
                    transfer_value_blinder,
                    transfer_sender_blinder,
                ),
                (
                    change_note,
                    change_value,
                    JubJubScalar::zero(),
                    change_sender_blinder,
                ),
            ],
            payload,
            sender_pk,
            signatures: (sig_a, sig_b),
        };

        let utx_bytes = utx.to_var_bytes();
        let deserialized_utx = UnprovenTransaction::from_slice(&utx_bytes[..])?;
        assert_eq!(utx.inputs, deserialized_utx.inputs);
        assert_eq!(utx.outputs, deserialized_utx.outputs);
        assert_eq!(utx.payload, deserialized_utx.payload);
        assert_eq!(utx.sender_pk, deserialized_utx.sender_pk);
        assert_eq!(utx.signatures, deserialized_utx.signatures);

        Ok(())
    }
}
