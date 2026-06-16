// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_bytes::Serializable as DuskSerializable;
use dusk_core::signatures::bls::PublicKey as AccountPublicKey;
use dusk_core::transfer::moonlight::Transaction as MoonlightTransaction;
use dusk_core::transfer::phoenix::Transaction as PhoenixTransaction;
use dusk_core::transfer::{
    Transaction as ProtocolTransaction, TransactionFormat,
};
use serde::Serialize;
use sha3::Digest;

use crate::hard_fork;

/// Number of bytes used by the outer ledger transaction header.
///
/// The header contains the transaction version, transaction type, and
/// length-prefix for the encoded protocol transaction.
pub(crate) const LEDGER_TRANSACTION_HEADER_BYTES: usize =
    std::mem::size_of::<u32>() * 3;

/// Canonical protocol transaction representation for a specific
/// [`TransactionFormat`].
#[derive(Debug, Clone)]
pub struct CanonicalTransaction {
    protocol: ProtocolTransaction,
    format: TransactionFormat,
}

impl CanonicalTransaction {
    /// Canonicalizes a native protocol transaction using the ledger format for
    /// `block_height`.
    pub fn canonicalize_for_ledger(
        protocol: ProtocolTransaction,
        block_height: u64,
    ) -> Self {
        Self::canonicalize(
            protocol,
            hard_fork::ledger_tx_format_at(block_height),
        )
    }

    /// Canonicalizes a native protocol transaction using the ingress format for
    /// `block_height`.
    pub fn canonicalize_for_ingress(
        protocol: ProtocolTransaction,
        block_height: u64,
    ) -> Self {
        Self::canonicalize(
            protocol,
            hard_fork::ingress_tx_format_at(block_height),
        )
    }

    /// Decodes transaction bytes for live ingress at `block_height`.
    ///
    /// Live network ingress accepts the currently supported transport
    /// envelopes (`Aegis` and `Boreas`) and normalizes them to the canonical
    /// ingress format for that height. Historical `PreAegis` encodings remain
    /// replay-only and are rejected here.
    pub fn decode_for_ingress(
        bytes: &[u8],
        block_height: u64,
    ) -> Result<Self, dusk_bytes::Error> {
        let decoded = Self::decode_any(bytes)?;

        if decoded.format() == TransactionFormat::PreAegis {
            return Err(dusk_bytes::Error::InvalidData);
        }

        Ok(decoded.reformat_for_ingress(block_height))
    }

    /// Decodes transaction bytes using the ledger format for `block_height`.
    ///
    /// This is the format used by block and ledger replay at that height.
    pub fn decode_for_ledger(
        bytes: &[u8],
        block_height: u64,
    ) -> Result<Self, dusk_bytes::Error> {
        Self::decode_with_selected_format(
            bytes,
            hard_fork::ledger_tx_format_at(block_height),
        )
    }

    /// Decodes transaction bytes using the format encoded by the payload.
    pub fn decode_any(bytes: &[u8]) -> Result<Self, dusk_bytes::Error> {
        let decoded = ProtocolTransaction::decode_any(bytes)?;
        Ok(Self::from_parts(decoded.transaction, decoded.format))
    }

    pub fn canonicalize(
        protocol: ProtocolTransaction,
        format: TransactionFormat,
    ) -> Self {
        Self::from_parts(protocol, format)
    }

    fn decode_with_selected_format(
        bytes: &[u8],
        format: TransactionFormat,
    ) -> Result<Self, dusk_bytes::Error> {
        let decoded = ProtocolTransaction::decode_with_format(format, bytes)?;
        Ok(Self::from_parts(decoded.transaction, decoded.format))
    }

    fn from_parts(
        protocol: ProtocolTransaction,
        format: TransactionFormat,
    ) -> Self {
        Self { protocol, format }
    }

    fn digest_bytes(
        protocol: &ProtocolTransaction,
        format: TransactionFormat,
    ) -> [u8; 32] {
        let tx_bytes = protocol.blob_to_memo().map_or_else(
            || protocol.encode_for_format(format),
            |mut blob_tx| {
                let _ = blob_tx.strip_blobs();
                blob_tx.encode_for_format(format)
            },
        );

        sha3::Sha3_256::digest(tx_bytes).into()
    }

    /// Returns the native protocol transaction.
    pub fn protocol(&self) -> &ProtocolTransaction {
        &self.protocol
    }

    /// Returns the serialization format used for the protocol transaction.
    pub fn format(&self) -> TransactionFormat {
        self.format
    }

    /// Reformats a canonical transaction to the ingress format active at
    /// `block_height`.
    pub fn reformat_for_ingress(&self, block_height: u64) -> Self {
        let expected = hard_fork::ingress_tx_format_at(block_height);

        if self.format() == expected {
            return self.clone();
        }

        Self::canonicalize(self.protocol.clone(), expected)
    }

    /// Encodes the native protocol transaction using its selected format.
    pub fn protocol_bytes(&self) -> Vec<u8> {
        self.protocol.encode_for_format(self.format)
    }

    /// Computes the transaction ID.
    ///
    /// This is the protocol-level transaction hash used to identify the
    /// transaction.
    pub fn id(&self) -> [u8; 32] {
        self.protocol.hash().to_bytes()
    }

    /// Computes the ledger digest of the transaction data.
    ///
    /// This digest hashes the encoded transaction data used by the ledger state
    /// root. For blob transactions, sidecar data is stripped first.
    pub fn digest(&self) -> [u8; 32] {
        Self::digest_bytes(&self.protocol, self.format)
    }

    pub fn gas_price(&self) -> u64 {
        self.protocol.gas_price()
    }

    pub fn to_spend_ids(&self) -> Vec<SpendingId> {
        match &self.protocol {
            ProtocolTransaction::Phoenix(p) => p
                .nullifiers()
                .iter()
                .map(|n| SpendingId::Nullifier(n.to_bytes()))
                .collect(),
            ProtocolTransaction::Moonlight(m) => {
                vec![SpendingId::AccountNonce(*m.sender(), m.nonce())]
            }
        }
    }

    pub fn next_spending_id(&self) -> Option<SpendingId> {
        match &self.protocol {
            ProtocolTransaction::Phoenix(_) => None,
            ProtocolTransaction::Moonlight(m) => {
                Some(SpendingId::AccountNonce(*m.sender(), m.nonce() + 1))
            }
        }
    }
}

/// Ledger transaction wrapper around a [`CanonicalTransaction`].
#[derive(Debug, Clone)]
pub struct LedgerTransaction {
    pub version: u32,
    pub r#type: u32,
    canonical: CanonicalTransaction,
}

impl LedgerTransaction {
    /// Returns the encoded ledger transaction size, including the outer header.
    pub fn size(&self) -> usize {
        LEDGER_TRANSACTION_HEADER_BYTES + self.protocol_bytes().len()
    }
}

impl From<CanonicalTransaction> for LedgerTransaction {
    fn from(value: CanonicalTransaction) -> Self {
        Self {
            r#type: 1,
            version: 1,
            canonical: value,
        }
    }
}

/// A spent transaction is a transaction that has been included in a block and
/// was executed.
#[derive(Debug, Clone, Serialize)]
pub struct SpentTransaction {
    /// The transaction that was executed.
    pub inner: LedgerTransaction,
    /// The height of the block in which the transaction was included.
    pub block_height: u64,
    /// The amount of gas that was spent during the execution of the
    /// transaction.
    pub gas_spent: u64,
    /// An optional error message if the transaction execution yielded an
    /// error.
    pub err: Option<String>,
}

impl SpentTransaction {
    /// Returns the underlying public transaction, if it is one. Otherwise,
    /// returns `None`.
    pub fn public(&self) -> Option<&MoonlightTransaction> {
        match self.inner.protocol() {
            ProtocolTransaction::Moonlight(public_tx) => Some(public_tx),
            _ => None,
        }
    }

    /// Returns the underlying shielded transaction, if it is one. Otherwise,
    /// returns `None`.
    pub fn shielded(&self) -> Option<&PhoenixTransaction> {
        match self.inner.protocol() {
            ProtocolTransaction::Phoenix(shielded_tx) => Some(shielded_tx),
            _ => None,
        }
    }
}

impl LedgerTransaction {
    pub fn decode_for_ingress(
        bytes: &[u8],
        block_height: u64,
    ) -> Result<Self, dusk_bytes::Error> {
        CanonicalTransaction::decode_for_ingress(bytes, block_height)
            .map(Into::into)
    }

    pub fn decode_for_ledger(
        bytes: &[u8],
        block_height: u64,
    ) -> Result<Self, dusk_bytes::Error> {
        CanonicalTransaction::decode_for_ledger(bytes, block_height)
            .map(Into::into)
    }

    pub fn decode_any(bytes: &[u8]) -> Result<Self, dusk_bytes::Error> {
        CanonicalTransaction::decode_any(bytes).map(Into::into)
    }

    /// Builds a ledger transaction from a native protocol transaction and an
    /// explicit format.
    pub fn from_protocol_with_format(
        protocol: ProtocolTransaction,
        format: TransactionFormat,
    ) -> Self {
        CanonicalTransaction::canonicalize(protocol, format).into()
    }

    pub fn from_protocol_for_ledger(
        protocol: ProtocolTransaction,
        block_height: u64,
    ) -> Self {
        CanonicalTransaction::canonicalize_for_ledger(protocol, block_height)
            .into()
    }

    pub fn from_protocol_for_ingress(
        protocol: ProtocolTransaction,
        block_height: u64,
    ) -> Self {
        CanonicalTransaction::canonicalize_for_ingress(protocol, block_height)
            .into()
    }

    /// Reformats a ledger transaction to the ledger format active at
    /// `block_height`.
    pub fn reformat_for_ledger(&self, block_height: u64) -> Self {
        let expected = hard_fork::ledger_tx_format_at(block_height);

        if self.format() == expected {
            return self.clone();
        }

        Self::from_protocol_with_format(self.protocol().clone(), expected)
    }

    /// Reformats a ledger transaction to the ingress format active at
    /// `block_height`.
    pub fn reformat_for_ingress(&self, block_height: u64) -> Self {
        let expected = hard_fork::ingress_tx_format_at(block_height);

        if self.format() == expected {
            return self.clone();
        }

        Self::from_protocol_with_format(self.protocol().clone(), expected)
    }

    pub fn format(&self) -> TransactionFormat {
        self.canonical.format()
    }

    pub fn canonical(&self) -> &CanonicalTransaction {
        &self.canonical
    }

    pub fn protocol(&self) -> &ProtocolTransaction {
        self.canonical.protocol()
    }

    pub fn protocol_bytes(&self) -> Vec<u8> {
        self.canonical.protocol_bytes()
    }

    /// Computes the hash digest of the entire transaction data.
    ///
    /// This method returns the Sha3 256 digest of the entire
    /// transaction in its serialized form. If the transaction is a blob
    /// transaction, the sidecar is stripped
    ///
    /// The digest hash is currently only being used in the merkle tree.
    ///
    /// ### Returns
    /// An array of 32 bytes representing the hash of the transaction.
    pub fn digest(&self) -> [u8; 32] {
        self.canonical.digest()
    }

    /// Computes the transaction ID.
    ///
    /// The transaction ID is a unique identifier for the transaction.
    /// Unlike the [`digest()`](#method.digest) method, which is computed over
    /// the entire transaction, the transaction ID is derived from specific
    /// fields of the transaction and serves as a unique identifier of the
    /// transaction itself.
    ///
    /// ### Returns
    /// An array of 32 bytes representing the transaction ID.
    pub fn id(&self) -> [u8; 32] {
        self.canonical.id()
    }

    pub fn gas_price(&self) -> u64 {
        self.protocol().gas_price()
    }

    pub fn to_spend_ids(&self) -> Vec<SpendingId> {
        self.canonical.to_spend_ids()
    }

    pub fn next_spending_id(&self) -> Option<SpendingId> {
        self.canonical.next_spending_id()
    }

    pub fn blob_mut(
        &mut self,
    ) -> Option<&mut Vec<dusk_core::transfer::data::BlobData>> {
        self.canonical.protocol.blob_mut()
    }

    pub fn strip_blobs(
        &mut self,
    ) -> Option<Vec<([u8; 32], dusk_core::transfer::data::BlobSidecar)>> {
        self.canonical.protocol.strip_blobs()
    }
}

impl PartialEq<Self> for LedgerTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.r#type == other.r#type
            && self.version == other.version
            && self.format() == other.format()
            && self.id() == other.id()
    }
}

impl Eq for LedgerTransaction {}

impl PartialEq<Self> for SpentTransaction {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner && self.gas_spent == other.gas_spent
    }
}

impl Eq for SpentTransaction {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpendingId {
    Nullifier([u8; 32]),
    AccountNonce(AccountPublicKey, u64),
}

impl SpendingId {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            SpendingId::Nullifier(n) => n.to_vec(),
            SpendingId::AccountNonce(account, nonce) => {
                let mut id = account.to_bytes().to_vec();
                id.extend_from_slice(&nonce.to_le_bytes());
                id
            }
        }
    }

    pub fn next(&self) -> Option<SpendingId> {
        match self {
            SpendingId::Nullifier(_) => None,
            SpendingId::AccountNonce(account, nonce) => {
                Some(SpendingId::AccountNonce(*account, nonce + 1))
            }
        }
    }
}

#[cfg(any(feature = "faker", test))]
pub mod faker {
    use dusk_core::transfer::data::{ContractCall, TransactionData};
    use dusk_core::transfer::phoenix::{
        Fee, Note, Payload as PhoenixPayload, PublicKey as PhoenixPublicKey,
        SecretKey as PhoenixSecretKey, Transaction as PhoenixTransaction,
        TxSkeleton,
    };
    use dusk_core::{BlsScalar, JubJubScalar};
    use rand::Rng;

    use super::*;
    use crate::ledger::Dummy;

    impl<T> Dummy<T> for LedgerTransaction {
        fn dummy_with_rng<R: Rng + ?Sized>(_config: &T, _rng: &mut R) -> Self {
            gen_dummy_tx(1_000_000)
        }
    }

    impl<T> Dummy<T> for SpentTransaction {
        fn dummy_with_rng<R: Rng + ?Sized>(_config: &T, _rng: &mut R) -> Self {
            let tx = LedgerTransaction::from_protocol_with_format(
                gen_dummy_tx(1_000_000).protocol().clone(),
                TransactionFormat::PreAegis,
            );
            SpentTransaction {
                inner: tx,
                block_height: 0,
                gas_spent: 3,
                err: Some("error".to_string()),
            }
        }
    }

    /// Generates a decodable transaction from a fixed blob with a specified
    /// gas price.
    pub fn gen_dummy_tx(gas_price: u64) -> LedgerTransaction {
        let pk = PhoenixPublicKey::from(&PhoenixSecretKey::new(
            JubJubScalar::from(42u64),
            JubJubScalar::from(42u64),
        ));
        let gas_limit = 1;

        let fee = Fee::deterministic(
            &JubJubScalar::from(5u64),
            &pk,
            gas_limit,
            gas_price,
            &[JubJubScalar::from(9u64), JubJubScalar::from(10u64)],
        );

        let tx_skeleton = TxSkeleton {
            root: BlsScalar::from(12345u64),
            nullifiers: vec![
                BlsScalar::from(1u64),
                BlsScalar::from(2u64),
                BlsScalar::from(3u64),
            ],
            outputs: [Note::empty(), Note::empty()],
            max_fee: gas_price * gas_limit,
            deposit: 0,
        };

        let contract_call = ContractCall::new([21; 32], "some_method");

        let payload = PhoenixPayload {
            chain_id: 0xFA,
            tx_skeleton,
            fee,
            data: Some(TransactionData::Call(contract_call)),
        };
        let proof = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];

        let tx: ProtocolTransaction =
            PhoenixTransaction::from_payload_and_proof(payload, proof).into();

        LedgerTransaction::from_protocol_with_format(
            tx,
            TransactionFormat::Aegis,
        )
    }
}

#[cfg(test)]
mod tests {
    use dusk_core::transfer::TransactionFormat;

    use super::faker::gen_dummy_tx;
    use super::*;

    #[test]
    fn decode_for_ingress_accepts_aegis_and_normalizes_to_boreas() {
        let tx = gen_dummy_tx(10);
        let bytes = tx.protocol_bytes();

        let decoded =
            CanonicalTransaction::decode_for_ingress(&bytes, u64::MAX)
                .expect("aegis bytes should decode for boreas ingress");

        assert_eq!(decoded.format(), TransactionFormat::Boreas);
        assert_eq!(decoded.id(), tx.id());
    }

    #[test]
    fn decode_for_ingress_accepts_boreas_and_normalizes_to_aegis() {
        let tx = gen_dummy_tx(10);
        let bytes = tx.protocol().encode_for_format(TransactionFormat::Boreas);

        let decoded = CanonicalTransaction::decode_for_ingress(&bytes, 1)
            .expect("boreas bytes should decode for aegis ingress");

        assert_eq!(decoded.format(), TransactionFormat::Aegis);
        assert_eq!(decoded.id(), tx.id());
    }

    #[test]
    fn reformat_for_ingress_preserves_tx_identity() {
        let tx = gen_dummy_tx(10);
        let reformatted = tx.reformat_for_ingress(u64::MAX);

        assert_eq!(reformatted.format(), TransactionFormat::Boreas);
        assert_eq!(reformatted.id(), tx.id());
    }

    #[test]
    fn reformat_for_ingress_preserves_tx_identity_before_boreas() {
        let tx = LedgerTransaction::from_protocol_with_format(
            gen_dummy_tx(10).protocol().clone(),
            TransactionFormat::Boreas,
        );
        let reformatted = tx.reformat_for_ingress(1);

        assert_eq!(reformatted.format(), TransactionFormat::Aegis);
        assert_eq!(reformatted.id(), tx.id());
    }
}
