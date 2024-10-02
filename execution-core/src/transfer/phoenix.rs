// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Types related to the phoenix transaction model of Dusk's transfer contract.

use alloc::vec::Vec;
use core::cmp;
use core::fmt::Debug;

use bytecheck::CheckBytes;
use dusk_bytes::{DeserializableSlice, Error as BytesError, Serializable};
use dusk_poseidon::{Domain, Hash};
use ff::Field;
use rand::{CryptoRng, RngCore};
use rkyv::{Archive, Deserialize, Serialize};

use crate::{
    signatures::schnorr::{
        SecretKey as SchnorrSecretKey, Signature as SchnorrSignature,
    },
    transfer::{
        data::{
            ContractBytecode, ContractCall, ContractDeploy, TransactionData,
            MAX_MEMO_SIZE,
        },
        MINIMUM_GAS_PRICE,
    },
    BlsScalar, Error, JubJubAffine, JubJubScalar,
};

// phoenix types
pub use phoenix_circuits::{InputNoteInfo, OutputNoteInfo, TxCircuit};
pub use phoenix_core::{
    value_commitment, Error as CoreError, Note, PublicKey, SecretKey, Sender,
    StealthAddress, TxSkeleton, ViewKey, NOTE_VAL_ENC_SIZE, OUTPUT_NOTES,
};

/// The depth of the merkle tree of notes stored in the transfer-contract.
pub const NOTES_TREE_DEPTH: usize = 17;
/// The arity of the merkle tree of notes stored in the transfer-contract.
pub use poseidon_merkle::ARITY as NOTES_TREE_ARITY;
/// The merkle tree of notes stored in the transfer-contract.
pub type NotesTree = poseidon_merkle::Tree<(), NOTES_TREE_DEPTH>;
/// The merkle opening for a note-hash in the merkle tree of notes.
pub type NoteOpening = poseidon_merkle::Opening<(), NOTES_TREE_DEPTH>;
/// the tree item for the merkle-tree of notes stored in the transfer-contract.
pub type NoteTreeItem = poseidon_merkle::Item<()>;

/// A leaf of the merkle tree of notes stored in the transfer-contract.
#[derive(Debug, Clone, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct NoteLeaf {
    /// The height of the block when the note was inserted in the tree.
    pub block_height: u64,
    /// The note inserted in the tree.
    pub note: Note,
}

impl AsRef<Note> for NoteLeaf {
    fn as_ref(&self) -> &Note {
        &self.note
    }
}

/// Ord compares positions, not values, note values need to be decrypted first
impl cmp::Ord for NoteLeaf {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.note.pos().cmp(other.note.pos())
    }
}

impl cmp::PartialOrd for NoteLeaf {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Label used for the ZK transcript initialization. Must be the same for prover
/// and verifier.
pub const TRANSCRIPT_LABEL: &[u8] = b"dusk-network";

/// Phoenix transaction.
#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Transaction {
    payload: Payload,
    proof: Vec<u8>,
}

impl PartialEq for Transaction {
    fn eq(&self, other: &Self) -> bool {
        self.hash() == other.hash()
    }
}

impl Eq for Transaction {}

impl Transaction {
    /// Create a new phoenix transaction given the sender secret-key, receiver
    /// public-key, the input note positions in the transaction tree and the
    /// new output-notes.
    ///
    /// # Errors
    /// The creation of a transaction is not possible and will error if:
    /// - one of the input-notes doesn't belong to the `sender_sk`
    /// - the transaction input doesn't cover the transaction costs
    /// - the `inputs` vector is either empty or larger than 4 elements
    /// - the `inputs` vector contains duplicate `Note`s
    /// - the `prover` is implemented incorrectly
    /// - the memo, if given, is too large
    #[allow(clippy::too_many_lines)]
    #[allow(clippy::too_many_arguments)]
    #[allow(clippy::similar_names)]
    pub fn new<R: RngCore + CryptoRng, P: Prove>(
        rng: &mut R,
        sender_sk: &SecretKey,
        change_pk: &PublicKey,
        receiver_pk: &PublicKey,
        inputs: Vec<(Note, NoteOpening)>,
        root: BlsScalar,
        transfer_value: u64,
        obfuscated_transaction: bool,
        deposit: u64,
        gas_limit: u64,
        gas_price: u64,
        chain_id: u8,
        data: Option<impl Into<TransactionData>>,
        prover: &P,
    ) -> Result<Self, Error> {
        let data = data.map(Into::into);

        if let Some(TransactionData::Memo(memo)) = data.as_ref() {
            if memo.len() > MAX_MEMO_SIZE {
                return Err(Error::MemoTooLarge(memo.len()));
            }
        }

        let sender_pk = PublicKey::from(sender_sk);
        let sender_vk = ViewKey::from(sender_sk);

        // get input note values, value-blinders and nullifiers
        let input_len = inputs.len();
        let mut input_values = Vec::with_capacity(input_len);
        let mut input_value_blinders = Vec::with_capacity(input_len);
        let mut input_nullifiers = Vec::with_capacity(input_len);
        for (note, _opening) in &inputs {
            let note_nullifier = note.gen_nullifier(sender_sk);
            for nullifier in &input_nullifiers {
                if note_nullifier == *nullifier {
                    return Err(Error::Replay);
                }
            }
            input_nullifiers.push(note_nullifier);
            input_values.push(note.value(Some(&sender_vk))?);
            input_value_blinders.push(note.value_blinder(Some(&sender_vk))?);
        }
        let input_value: u64 = input_values.iter().sum();

        // --- Create the transaction payload

        // Set the fee.
        let fee = Fee::new(rng, change_pk, gas_limit, gas_price);
        let max_fee = fee.max_fee();

        if input_value < transfer_value + max_fee + deposit {
            return Err(Error::InsufficientBalance);
        }

        // Check if the gas price is lower than the minimum: 1
        if gas_price < MINIMUM_GAS_PRICE {
            return Err(Error::GasPriceTooLow);
        }

        // Generate output notes:
        let transfer_value_blinder = if obfuscated_transaction {
            JubJubScalar::random(&mut *rng)
        } else {
            JubJubScalar::zero()
        };
        let transfer_sender_blinder = [
            JubJubScalar::random(&mut *rng),
            JubJubScalar::random(&mut *rng),
        ];
        let change_sender_blinder = [
            JubJubScalar::random(&mut *rng),
            JubJubScalar::random(&mut *rng),
        ];
        let transfer_note = if obfuscated_transaction {
            Note::obfuscated(
                rng,
                &sender_pk,
                receiver_pk,
                transfer_value,
                transfer_value_blinder,
                transfer_sender_blinder,
            )
        } else {
            Note::transparent(
                rng,
                &sender_pk,
                receiver_pk,
                transfer_value,
                transfer_sender_blinder,
            )
        };
        // The change note should have the value of the input note, minus what
        // is maximally spent.
        let change_value = input_value - transfer_value - max_fee - deposit;
        let change_value_blinder = JubJubScalar::random(&mut *rng);
        let change_note = Note::obfuscated(
            rng,
            &sender_pk,
            change_pk,
            change_value,
            change_value_blinder,
            change_sender_blinder,
        );
        let outputs = [transfer_note.clone(), change_note.clone()];

        // Now we can set the tx-skeleton, payload and get the payload-hash
        let tx_skeleton = TxSkeleton {
            root,
            // we also need the nullifiers for the tx-circuit, hence the clone
            nullifiers: input_nullifiers.clone(),
            outputs,
            max_fee,
            deposit,
        };
        let payload = Payload {
            chain_id,
            tx_skeleton,
            fee,
            data,
        };
        let payload_hash = payload.hash();

        // --- Create the transaction proof

        // Create a vector with all the information for the input-notes
        let mut input_notes_info = Vec::with_capacity(input_len);
        inputs
            .into_iter()
            .zip(input_nullifiers)
            .zip(input_values)
            .zip(input_value_blinders)
            .for_each(
                |(
                    (((note, merkle_opening), nullifier), value),
                    value_blinder,
                )| {
                    let note_sk = sender_sk.gen_note_sk(note.stealth_address());
                    let note_pk_p = JubJubAffine::from(
                        crate::GENERATOR_NUMS_EXTENDED * note_sk.as_ref(),
                    );
                    let signature = note_sk.sign_double(rng, payload_hash);
                    input_notes_info.push(InputNoteInfo {
                        merkle_opening,
                        note,
                        note_pk_p,
                        value,
                        value_blinder,
                        nullifier,
                        signature,
                    });
                },
            );

        // Create the information for the output-notes
        let transfer_value_commitment =
            value_commitment(transfer_value, transfer_value_blinder);
        let transfer_note_sender_enc = match transfer_note.sender() {
            Sender::Encryption(enc) => enc,
            Sender::ContractInfo(_) => unreachable!("The sender is encrypted"),
        };
        let change_value_commitment =
            value_commitment(change_value, change_value_blinder);
        let change_note_sender_enc = match change_note.sender() {
            Sender::Encryption(enc) => enc,
            Sender::ContractInfo(_) => unreachable!("The sender is encrypted"),
        };
        let output_notes_info = [
            OutputNoteInfo {
                value: transfer_value,
                value_commitment: transfer_value_commitment,
                value_blinder: transfer_value_blinder,
                note_pk: JubJubAffine::from(
                    transfer_note.stealth_address().note_pk().as_ref(),
                ),
                sender_enc: *transfer_note_sender_enc,
                sender_blinder: transfer_sender_blinder,
            },
            OutputNoteInfo {
                value: change_value,
                value_commitment: change_value_commitment,
                value_blinder: change_value_blinder,
                note_pk: JubJubAffine::from(
                    change_note.stealth_address().note_pk().as_ref(),
                ),
                sender_enc: *change_note_sender_enc,
                sender_blinder: change_sender_blinder,
            },
        ];

        // Sign the payload hash using both 'a' and 'b' of the sender_sk
        let schnorr_sk_a = SchnorrSecretKey::from(sender_sk.a());
        let sig_a = schnorr_sk_a.sign(rng, payload_hash);
        let schnorr_sk_b = SchnorrSecretKey::from(sender_sk.b());
        let sig_b = schnorr_sk_b.sign(rng, payload_hash);

        Ok(Self {
            payload,
            proof: prover.prove(
                &TxCircuitVec {
                    input_notes_info,
                    output_notes_info,
                    payload_hash,
                    root,
                    deposit,
                    max_fee,
                    sender_pk,
                    signatures: (sig_a, sig_b),
                }
                .to_var_bytes(),
            )?,
        })
    }

    /// Creates a new phoenix transaction given the [`Payload`] and proof. Note
    /// that this function doesn't guarantee that the proof matches the
    /// payload, if possible use [`Self::new`] instead.
    #[must_use]
    pub fn from_payload_and_proof(payload: Payload, proof: Vec<u8>) -> Self {
        Self { payload, proof }
    }

    /// Replaces the inner `proof` bytes for a given `proof`.
    ///
    /// This can be used to delegate the proof generation after a
    /// [`Transaction`] is created.
    /// In order to do that, the transaction would be created using the
    /// serialized circuit-bytes for the proof-field. Those bytes can be
    /// sent to a 3rd-party prover-service that generates the proof-bytes
    /// and sends them back. The proof-bytes will then replace the
    /// circuit-bytes in the transaction using this function.
    pub fn set_proof(&mut self, proof: Vec<u8>) {
        self.proof = proof;
    }

    /// The proof of the transaction.
    #[must_use]
    pub fn proof(&self) -> &[u8] {
        &self.proof
    }

    /// The payload-hash of the transaction used as input in the
    /// phoenix-circuit.
    #[must_use]
    pub fn payload_hash(&self) -> BlsScalar {
        self.payload.hash()
    }

    /// Returns the nullifiers of the transaction.
    #[must_use]
    pub fn nullifiers(&self) -> &[BlsScalar] {
        &self.payload.tx_skeleton.nullifiers
    }

    /// Return the root of the notes tree.
    #[must_use]
    pub fn root(&self) -> &BlsScalar {
        &self.payload.tx_skeleton.root
    }

    /// Return the output notes of the transaction.
    #[must_use]
    pub fn outputs(&self) -> &[Note; OUTPUT_NOTES] {
        &self.payload.tx_skeleton.outputs
    }

    /// Return the fee for the transaction.
    #[must_use]
    pub fn fee(&self) -> &Fee {
        &self.payload.fee
    }

    /// Return the stealth address for returning funds for Phoenix transactions
    /// as specified in the fee.
    #[must_use]
    pub fn stealth_address(&self) -> &StealthAddress {
        &self.payload.fee.stealth_address
    }

    /// Returns the sender data for Phoenix transactions as specified in the
    /// fee.
    #[must_use]
    pub fn sender(&self) -> &Sender {
        &self.payload.fee.sender
    }

    /// Returns the gas limit of the transaction.
    #[must_use]
    pub fn gas_limit(&self) -> u64 {
        self.payload.fee.gas_limit
    }

    /// Returns the gas price of the transaction.
    #[must_use]
    pub fn gas_price(&self) -> u64 {
        self.payload.fee.gas_price
    }

    /// Returns the chain ID of the transaction.
    #[must_use]
    pub fn chain_id(&self) -> u8 {
        self.payload.chain_id
    }

    /// Returns the max fee to be spend by the transaction.
    #[must_use]
    pub fn max_fee(&self) -> u64 {
        self.payload.tx_skeleton.max_fee
    }

    /// Returns the deposit of the transaction.
    #[must_use]
    pub fn deposit(&self) -> u64 {
        self.payload.tx_skeleton.deposit
    }

    /// Return the contract call data, if there is any.
    #[must_use]
    pub fn call(&self) -> Option<&ContractCall> {
        #[allow(clippy::match_wildcard_for_single_variants)]
        match self.data()? {
            TransactionData::Call(ref c) => Some(c),
            _ => None,
        }
    }

    /// Return the contract deploy data, if there is any.
    #[must_use]
    pub fn deploy(&self) -> Option<&ContractDeploy> {
        #[allow(clippy::match_wildcard_for_single_variants)]
        match self.data()? {
            TransactionData::Deploy(ref d) => Some(d),
            _ => None,
        }
    }

    /// Returns the memo used with the transaction, if any.
    #[must_use]
    pub fn memo(&self) -> Option<&[u8]> {
        match self.data()? {
            TransactionData::Memo(memo) => Some(memo),
            _ => None,
        }
    }

    /// Returns the transaction data, if it exists.
    #[must_use]
    fn data(&self) -> Option<&TransactionData> {
        self.payload.data.as_ref()
    }

    /// Creates a modified clone of this transaction if it contains data for
    /// deployment, clones all fields except for the bytecode' 'bytes' part.
    /// Returns none if the transaction is not a deployment transaction.
    #[must_use]
    pub fn strip_off_bytecode(&self) -> Option<Self> {
        let deploy = self.deploy()?;

        let stripped_deploy = TransactionData::Deploy(ContractDeploy {
            owner: deploy.owner.clone(),
            init_args: deploy.init_args.clone(),
            bytecode: ContractBytecode {
                hash: deploy.bytecode.hash,
                bytes: Vec::new(),
            },
            nonce: deploy.nonce,
        });

        let mut stripped_transaction = self.clone();
        stripped_transaction.payload.data = Some(stripped_deploy);

        Some(stripped_transaction)
    }

    /// Serialize the `Transaction` into a variable length byte buffer.
    #[must_use]
    pub fn to_var_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        let payload_bytes = self.payload.to_var_bytes();
        bytes.extend((payload_bytes.len() as u64).to_bytes());
        bytes.extend(payload_bytes);

        bytes.extend((self.proof.len() as u64).to_bytes());
        bytes.extend(&self.proof);

        bytes
    }

    /// Deserialize the Transaction from a bytes buffer.
    ///
    /// # Errors
    /// Errors when the bytes are not canonical.
    pub fn from_slice(buf: &[u8]) -> Result<Self, BytesError> {
        let mut buf = buf;

        let payload_len = usize::try_from(u64::from_reader(&mut buf)?)
            .map_err(|_| BytesError::InvalidData)?;

        if buf.len() < payload_len {
            return Err(BytesError::InvalidData);
        }
        let (payload_buf, new_buf) = buf.split_at(payload_len);

        let payload = Payload::from_slice(payload_buf)?;
        buf = new_buf;

        let proof_len = usize::try_from(u64::from_reader(&mut buf)?)
            .map_err(|_| BytesError::InvalidData)?;
        let proof = buf[..proof_len].into();

        Ok(Self { payload, proof })
    }

    /// Return input bytes to hash the Transaction.
    ///
    /// Note: The result of this function is *only* meant to be used as an input
    /// for hashing and *cannot* be used to deserialize the `Transaction` again.
    #[must_use]
    pub fn to_hash_input_bytes(&self) -> Vec<u8> {
        let mut bytes = self.payload.to_hash_input_bytes();
        bytes.extend(&self.proof);
        bytes
    }

    /// Create the `Transaction`-hash.
    #[must_use]
    pub fn hash(&self) -> BlsScalar {
        BlsScalar::hash_to_scalar(&self.to_hash_input_bytes())
    }

    /// Return the public input to be used in the phoenix-transaction circuit
    /// verification
    ///
    /// These are:
    /// - `payload_hash`
    /// - `root`
    /// - `[nullifier; I]`
    /// - `[output_value_commitment; 2]`
    /// - `max_fee`
    /// - `deposit`
    /// - `(npk_out_0, npk_out_1)`
    /// - `(enc_A_npk_out_0, enc_B_npk_out_0)`
    /// - `(enc_A_npk_out_1, enc_B_npk_out_1)`
    ///
    /// # Panics
    /// Panics if one of the output-notes doesn't have the sender set to
    /// [`Sender::Encryption`].
    #[must_use]
    pub fn public_inputs(&self) -> Vec<BlsScalar> {
        let tx_skeleton = &self.payload.tx_skeleton;

        // retrieve the number of input and output notes
        let input_len = tx_skeleton.nullifiers.len();
        let output_len = tx_skeleton.outputs.len();

        let size =
            // payload-hash and root
            1 + 1
            // nullifiers
            + input_len
            // output-notes value-commitment
            + 2 * output_len
            // max-fee and deposit
            + 1 + 1
            // output-notes public-keys
            + 2 * output_len
            // sender-encryption for both output-notes
            + 2 * 4 * output_len;
        // build the public input vector
        let mut pis = Vec::<BlsScalar>::with_capacity(size);
        pis.push(self.payload.hash());
        pis.push(tx_skeleton.root);
        pis.extend(tx_skeleton.nullifiers().iter());
        tx_skeleton.outputs().iter().for_each(|note| {
            let value_commitment = note.value_commitment();
            pis.push(value_commitment.get_u());
            pis.push(value_commitment.get_v());
        });
        pis.push(tx_skeleton.max_fee().into());
        pis.push(tx_skeleton.deposit().into());
        tx_skeleton.outputs().iter().for_each(|note| {
            let note_pk =
                JubJubAffine::from(note.stealth_address().note_pk().as_ref());
            pis.push(note_pk.get_u());
            pis.push(note_pk.get_v());
        });
        tx_skeleton.outputs().iter().for_each(|note| {
            match note.sender() {
                Sender::Encryption(sender_enc) => {
                    pis.push(sender_enc[0].0.get_u());
                    pis.push(sender_enc[0].0.get_v());
                    pis.push(sender_enc[0].1.get_u());
                    pis.push(sender_enc[0].1.get_v());
                    pis.push(sender_enc[1].0.get_u());
                    pis.push(sender_enc[1].0.get_v());
                    pis.push(sender_enc[1].1.get_u());
                    pis.push(sender_enc[1].1.get_v());
                }
                Sender::ContractInfo(_) => {
                    panic!("All output-notes must provide a sender-encryption")
                }
            };
        });

        pis
    }
}

/// The transaction payload
#[derive(Debug, Clone, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Payload {
    /// ID of the chain for this transaction to execute on.
    pub chain_id: u8,
    /// Transaction skeleton used for the phoenix transaction.
    pub tx_skeleton: TxSkeleton,
    /// Data used to calculate the transaction fee.
    pub fee: Fee,
    /// Data to do a contract call, deployment, or insert a memo.
    pub data: Option<TransactionData>,
}

impl PartialEq for Payload {
    fn eq(&self, other: &Self) -> bool {
        self.hash() == other.hash()
    }
}

impl Eq for Payload {}

impl Payload {
    /// Serialize the `Payload` into a variable length byte buffer.
    #[must_use]
    pub fn to_var_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::from([self.chain_id]);

        // serialize the tx-skeleton
        let skeleton_bytes = self.tx_skeleton.to_var_bytes();
        bytes.extend((skeleton_bytes.len() as u64).to_bytes());
        bytes.extend(skeleton_bytes);

        // serialize the fee
        bytes.extend(self.fee.to_bytes());

        // serialize the contract call, deployment or memo, if present.
        match &self.data {
            Some(TransactionData::Call(call)) => {
                bytes.push(1);
                bytes.extend(call.to_var_bytes());
            }
            Some(TransactionData::Deploy(deploy)) => {
                bytes.push(2);
                bytes.extend(deploy.to_var_bytes());
            }
            Some(TransactionData::Memo(memo)) => {
                bytes.push(3);
                bytes.extend((memo.len() as u64).to_bytes());
                bytes.extend(memo);
            }
            _ => bytes.push(0),
        }

        bytes
    }

    /// Deserialize the Payload from a bytes buffer.
    ///
    /// # Errors
    /// Errors when the bytes are not canonical.
    pub fn from_slice(buf: &[u8]) -> Result<Self, BytesError> {
        let mut buf = buf;

        let chain_id = u8::from_reader(&mut buf)?;

        // deserialize the tx-skeleton
        #[allow(clippy::cast_possible_truncation)]
        let skeleton_len = usize::try_from(u64::from_reader(&mut buf)?)
            .map_err(|_| BytesError::InvalidData)?;
        let tx_skeleton = TxSkeleton::from_slice(buf)?;
        buf = &buf[skeleton_len..];

        // deserialize fee
        let fee = Fee::from_reader(&mut buf)?;

        // deserialize contract call, deploy data, or memo, if present
        let data = match u8::from_reader(&mut buf)? {
            0 => None,
            1 => Some(TransactionData::Call(ContractCall::from_slice(buf)?)),
            2 => {
                Some(TransactionData::Deploy(ContractDeploy::from_slice(buf)?))
            }
            3 => {
                // we only build for 64-bit so this truncation is impossible
                #[allow(clippy::cast_possible_truncation)]
                let size = u64::from_reader(&mut buf)? as usize;

                if buf.len() != size || size > MAX_MEMO_SIZE {
                    return Err(BytesError::InvalidData);
                }

                let memo = buf[..size].to_vec();
                Some(TransactionData::Memo(memo))
            }
            _ => {
                return Err(BytesError::InvalidData);
            }
        };

        Ok(Self {
            chain_id,
            tx_skeleton,
            fee,
            data,
        })
    }

    /// Return input bytes to hash the payload.
    ///
    /// Note: The result of this function is *only* meant to be used as an input
    /// for hashing and *cannot* be used to deserialize the `Payload` again.
    #[must_use]
    pub fn to_hash_input_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::from([self.chain_id]);

        bytes.extend(self.tx_skeleton.to_hash_input_bytes());

        match &self.data {
            Some(TransactionData::Deploy(d)) => {
                bytes.extend(&d.bytecode.to_hash_input_bytes());
                bytes.extend(&d.owner);
                if let Some(init_args) = &d.init_args {
                    bytes.extend(init_args);
                }
            }
            Some(TransactionData::Call(c)) => {
                bytes.extend(c.contract.as_bytes());
                bytes.extend(c.fn_name.as_bytes());
                bytes.extend(&c.fn_args);
            }
            Some(TransactionData::Memo(m)) => {
                bytes.extend(m);
            }
            None => {}
        }

        bytes
    }

    /// Create the `Payload`-hash to be used as an input to the
    /// phoenix-transaction circuit.
    #[must_use]
    pub fn hash(&self) -> BlsScalar {
        BlsScalar::hash_to_scalar(&self.to_hash_input_bytes())
    }
}

/// The Fee structure
#[derive(Debug, Clone, Copy, Archive, Serialize, Deserialize)]
#[archive_attr(derive(CheckBytes))]
pub struct Fee {
    /// Gas limit set for a phoenix transaction
    pub gas_limit: u64,
    /// Gas price set for a phoenix transaction
    pub gas_price: u64,
    /// Address to send the remainder note
    pub stealth_address: StealthAddress,
    /// Sender to use for the remainder
    pub sender: Sender,
}

impl PartialEq for Fee {
    fn eq(&self, other: &Self) -> bool {
        self.sender == other.sender && self.hash() == other.hash()
    }
}

impl Eq for Fee {}

impl Fee {
    /// Create a new Fee with inner randomness
    #[must_use]
    pub fn new<R: RngCore + CryptoRng>(
        rng: &mut R,
        pk: &PublicKey,
        gas_limit: u64,
        gas_price: u64,
    ) -> Self {
        let r = JubJubScalar::random(&mut *rng);

        let sender_blinder = [
            JubJubScalar::random(&mut *rng),
            JubJubScalar::random(&mut *rng),
        ];

        Self::deterministic(&r, pk, gas_limit, gas_price, &sender_blinder)
    }

    /// Create a new Fee without inner randomness
    #[must_use]
    pub fn deterministic(
        r: &JubJubScalar,
        pk: &PublicKey,
        gas_limit: u64,
        gas_price: u64,
        sender_blinder: &[JubJubScalar; 2],
    ) -> Self {
        let stealth_address = pk.gen_stealth_address(r);
        let sender =
            Sender::encrypt(stealth_address.note_pk(), pk, sender_blinder);

        Fee {
            gas_limit,
            gas_price,
            stealth_address,
            sender,
        }
    }

    /// Calculate the max-fee.
    #[must_use]
    pub fn max_fee(&self) -> u64 {
        self.gas_limit * self.gas_price
    }

    /// Return a hash represented by `H(gas_limit, gas_price, H([note_pk]))`
    #[must_use]
    pub fn hash(&self) -> BlsScalar {
        let npk = self.stealth_address.note_pk().as_ref().to_hash_inputs();

        let hash_inputs = [
            BlsScalar::from(self.gas_limit),
            BlsScalar::from(self.gas_price),
            npk[0],
            npk[1],
        ];
        Hash::digest(Domain::Other, &hash_inputs)[0]
    }

    /// Generates a remainder from the fee and the given gas consumed.
    ///
    /// If there is a deposit, it means that the deposit hasn't been picked up
    /// by the contract. In this case, it is added to the remainder note.
    #[must_use]
    pub fn gen_remainder_note(
        &self,
        gas_consumed: u64,
        deposit: Option<u64>,
    ) -> Note {
        // Consuming more gas than the limit provided should never occur, and
        // it's not the responsibility of the `Fee` to check that.
        // Here defensively ensure it's not panicking, capping the gas consumed
        // to the gas limit.
        let gas_consumed = cmp::min(gas_consumed, self.gas_limit);
        let gas_changes = (self.gas_limit - gas_consumed) * self.gas_price;

        Note::transparent_stealth(
            self.stealth_address,
            gas_changes + deposit.unwrap_or_default(),
            self.sender,
        )
    }
}

const SIZE: usize = 2 * u64::SIZE + StealthAddress::SIZE + Sender::SIZE;

impl Serializable<SIZE> for Fee {
    type Error = BytesError;

    /// Converts a Fee into it's byte representation
    #[must_use]
    fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];

        buf[..u64::SIZE].copy_from_slice(&self.gas_limit.to_bytes());
        let mut start = u64::SIZE;
        buf[start..start + u64::SIZE]
            .copy_from_slice(&self.gas_price.to_bytes());
        start += u64::SIZE;
        buf[start..start + StealthAddress::SIZE]
            .copy_from_slice(&self.stealth_address.to_bytes());
        start += StealthAddress::SIZE;
        buf[start..start + Sender::SIZE]
            .copy_from_slice(&self.sender.to_bytes());

        buf
    }

    /// Attempts to convert a byte representation of a fee into a `Fee`,
    /// failing if the input is invalid
    fn from_bytes(bytes: &[u8; Self::SIZE]) -> Result<Self, Self::Error> {
        let mut reader = &bytes[..];

        let gas_limit = u64::from_reader(&mut reader)?;
        let gas_price = u64::from_reader(&mut reader)?;
        let stealth_address = StealthAddress::from_reader(&mut reader)?;
        let sender = Sender::from_reader(&mut reader)?;

        Ok(Fee {
            gas_limit,
            gas_price,
            stealth_address,
            sender,
        })
    }
}

/// This struct mimics the [`TxCircuit`] but is not generic over the amount of
/// input-notes.
#[derive(Debug, Clone, PartialEq)]
pub struct TxCircuitVec {
    /// All information needed in relation to the transaction input-notes
    pub input_notes_info: Vec<InputNoteInfo<NOTES_TREE_DEPTH>>,
    /// All information needed in relation to the transaction output-notes
    pub output_notes_info: [OutputNoteInfo; OUTPUT_NOTES],
    /// The hash of the transaction-payload
    pub payload_hash: BlsScalar,
    /// The root of the tree of notes corresponding to the input-note openings
    pub root: BlsScalar,
    /// The deposit of the transaction, is zero if there is no deposit
    pub deposit: u64,
    /// The maximum fee that the transaction may spend
    pub max_fee: u64,
    /// The public key of the sender used for the sender-encryption
    pub sender_pk: PublicKey,
    /// The signature of the payload-hash signed with sk.a and sk.b
    pub signatures: (SchnorrSignature, SchnorrSignature),
}

impl TxCircuitVec {
    /// Serialize a [`TxCircuitVec`] into a vector of bytes.
    #[must_use]
    pub fn to_var_bytes(&self) -> Vec<u8> {
        let input_len = self.input_notes_info.len();

        let mut bytes = Vec::with_capacity(Self::size(input_len));

        // first serialize the amount of input-notes
        bytes.extend((input_len as u64).to_bytes());

        // then serialize the other fields
        for info in &self.input_notes_info {
            bytes.extend(info.to_var_bytes());
        }
        for info in &self.output_notes_info {
            bytes.extend(info.to_bytes());
        }
        bytes.extend(self.payload_hash.to_bytes());
        bytes.extend(self.root.to_bytes());
        bytes.extend(self.deposit.to_bytes());
        bytes.extend(self.max_fee.to_bytes());
        bytes.extend(self.sender_pk.to_bytes());
        bytes.extend(self.signatures.0.to_bytes());
        bytes.extend(self.signatures.1.to_bytes());

        bytes
    }

    /// Deserialize a [`TxCircuitVec`] from a slice of bytes.
    ///
    /// # Errors
    ///
    /// Will return [`dusk_bytes::Error`] in case of a deserialization error.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, BytesError> {
        let input_len = u64::from_slice(bytes)?;

        // the input-len is smaller than a u32::MAX
        #[allow(clippy::cast_possible_truncation)]
        if bytes.len() < Self::size(input_len as usize) {
            return Err(BytesError::BadLength {
                found: bytes.len(),
                expected: Self::size(input_len as usize),
            });
        }

        let bytes = &bytes[u64::SIZE..];
        let circuit: TxCircuitVec = match input_len {
            1 => TxCircuit::<NOTES_TREE_DEPTH, 1>::from_slice(bytes)?.into(),
            2 => TxCircuit::<NOTES_TREE_DEPTH, 2>::from_slice(bytes)?.into(),
            3 => TxCircuit::<NOTES_TREE_DEPTH, 3>::from_slice(bytes)?.into(),
            4 => TxCircuit::<NOTES_TREE_DEPTH, 4>::from_slice(bytes)?.into(),
            _ => return Err(BytesError::InvalidData),
        };

        Ok(circuit)
    }

    const fn size(input_len: usize) -> usize {
        u64::SIZE
            + input_len * InputNoteInfo::<NOTES_TREE_DEPTH>::SIZE
            + OUTPUT_NOTES * OutputNoteInfo::SIZE
            + 2 * BlsScalar::SIZE
            + 2 * u64::SIZE
            + PublicKey::SIZE
            + 2 * SchnorrSignature::SIZE
    }
}

impl<const I: usize> From<TxCircuit<NOTES_TREE_DEPTH, I>> for TxCircuitVec {
    fn from(circuit: TxCircuit<NOTES_TREE_DEPTH, I>) -> Self {
        TxCircuitVec {
            input_notes_info: circuit.input_notes_info.to_vec(),
            output_notes_info: circuit.output_notes_info,
            payload_hash: circuit.payload_hash,
            root: circuit.root,
            deposit: circuit.deposit,
            max_fee: circuit.max_fee,
            sender_pk: circuit.sender_pk,
            signatures: circuit.signatures,
        }
    }
}

/// This trait can be used to implement different methods to generate a proof
/// from the circuit-bytes.
pub trait Prove {
    /// Generate a transaction proof from all the information needed to create a
    /// tx-circuit.
    ///
    /// # Errors
    /// This function errors in case of an incorrect circuit or of an
    /// unobtainable prover-key.
    //
    // Note that the reference to `self` is needed to plug in a running client
    // when delegating the proof generation.
    fn prove(&self, tx_circuit_vec_bytes: &[u8]) -> Result<Vec<u8>, Error>;
}
