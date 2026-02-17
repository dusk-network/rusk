// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::Arc;

use async_trait::async_trait;
use node_data::bls::PublicKeyBytes;
use node_data::ledger::to_str;
use node_data::message::payload::{Candidate, GetResource, Inv};
use node_data::message::{
    ConsensusHeader, Message, Payload, SignedStepMessage, StepMessage,
    WireMessage,
};
use tokio::sync::Mutex;
use tracing::info;

use crate::commons::{Database, RoundUpdate};
use crate::config::{
    MAX_BLOCK_SIZE, MAX_NUMBER_OF_FAULTS, MAX_NUMBER_OF_TRANSACTIONS,
    is_emergency_iter,
};
use crate::errors::ConsensusError;
use crate::iteration_ctx::RoundCommittees;
use crate::merkle::merkle_root;
use crate::msg_handler::{MsgHandler, StepOutcome};
use crate::user::committee::Committee;

pub struct ProposalHandler<D: Database> {
    pub(crate) db: Arc<Mutex<D>>,
}

#[async_trait]
impl<D: Database> MsgHandler for ProposalHandler<D> {
    /// Verifies if msg is a valid new_block message.
    fn verify(
        &self,
        msg: &Message,
        round_committees: &RoundCommittees,
    ) -> Result<(), ConsensusError> {
        let p = Self::unwrap_msg(msg)?;
        let iteration = p.header().iteration;
        let generator = round_committees
            .get_generator(iteration)
            .expect("committee to be created before run");
        super::handler::verify_candidate_msg(p, &generator)?;

        Ok(())
    }

    /// Collects а Candidate message.
    async fn collect(
        &mut self,
        msg: Message,
        _ru: &RoundUpdate,
        _committee: &Committee,
        _generator: Option<PublicKeyBytes>,
        _round_committees: &RoundCommittees,
    ) -> Result<StepOutcome, ConsensusError> {
        // store candidate block
        let p = Self::unwrap_msg(&msg)?;
        self.db
            .lock()
            .await
            .store_candidate_block(p.candidate.clone())
            .await;

        info!(
            event = "New Candidate",
            hash = &to_str(&p.candidate.header().hash),
            round = p.candidate.header().height,
            iter = p.candidate.header().iteration,
            prev_block = &to_str(&p.candidate.header().prev_block_hash)
        );

        Ok(StepOutcome::Ready(msg))
    }

    async fn collect_from_past(
        &mut self,
        msg: Message,
        _committee: &Committee,
        _generator: Option<PublicKeyBytes>,
    ) -> Result<StepOutcome, ConsensusError> {
        let p = Self::unwrap_msg(&msg)?;

        self.db
            .lock()
            .await
            .store_candidate_block(p.candidate.clone())
            .await;

        info!(
            event = "New Candidate",
            hash = &to_str(&p.candidate.header().hash),
            round = p.candidate.header().height,
            iter = p.candidate.header().iteration,
            prev_block = &to_str(&p.candidate.header().prev_block_hash)
        );

        Ok(StepOutcome::Ready(msg))
    }

    /// Handles of an event of step execution timeout
    fn handle_timeout(
        &self,
        ru: &RoundUpdate,
        curr_iteration: u8,
    ) -> Option<Message> {
        if is_emergency_iter(curr_iteration) {
            // In Emergency Mode we request the Candidate from our peers
            // in case we arrived late and missed the votes

            let prev_block_hash = ru.hash();
            let round = ru.round;

            info!(
                event = "request candidate block",
                src = "emergency_iter",
                iteration = curr_iteration,
                prev_block_hash = to_str(&ru.hash())
            );

            let mut inv = Inv::new(1);
            inv.add_candidate_from_iteration(ConsensusHeader {
                prev_block_hash,
                round,
                iteration: curr_iteration,
            });
            let msg = GetResource::new(inv, None, u64::MAX, 0);
            return Some(msg.into());
        }

        None
    }
}

impl<D: Database> ProposalHandler<D> {
    pub(crate) fn new(db: Arc<Mutex<D>>) -> Self {
        Self { db }
    }

    fn unwrap_msg(msg: &Message) -> Result<&Candidate, ConsensusError> {
        match &msg.payload {
            Payload::Candidate(c) => Ok(c),
            _ => Err(ConsensusError::InvalidMsgType),
        }
    }
}

fn verify_candidate_msg(
    p: &Candidate,
    expected_generator: &PublicKeyBytes,
) -> Result<(), ConsensusError> {
    if expected_generator != p.sign_info().signer.bytes() {
        return Err(ConsensusError::NotCommitteeMember);
    }

    let candidate_size = p
        .candidate
        .size()
        .map_err(|_| ConsensusError::UnknownBlockSize)?;
    if candidate_size > MAX_BLOCK_SIZE {
        return Err(ConsensusError::InvalidBlockSize(candidate_size));
    }

    // Verify msg signature
    p.verify_signature()?;

    if p.consensus_header().prev_block_hash
        != p.candidate.header().prev_block_hash
    {
        return Err(ConsensusError::InvalidBlockHash);
    }

    // INFO: we verify the transaction number and the merkle roots here because
    // the signature only includes the header's hash, making 'txs' and 'faults'
    // fields malleable from an adversary. We then discard blocks with errors
    // related to these fields rather than propagating the message and vote
    // Invalid

    // Check number of transactions
    if p.candidate.txs().len() > MAX_NUMBER_OF_TRANSACTIONS {
        return Err(ConsensusError::TooManyTransactions(
            p.candidate.txs().len(),
        ));
    }

    // Verify tx_root
    let tx_digests: Vec<_> =
        p.candidate.txs().iter().map(|t| t.digest()).collect();
    let tx_root = merkle_root(&tx_digests[..]);
    if tx_root != p.candidate.header().txroot {
        return Err(ConsensusError::InvalidBlock);
    }

    // Check number of faults
    if p.candidate.faults().len() > MAX_NUMBER_OF_FAULTS {
        return Err(ConsensusError::TooManyFaults(p.candidate.faults().len()));
    }

    // Verify fault_root
    let fault_digests: Vec<_> =
        p.candidate.faults().iter().map(|t| t.digest()).collect();
    let fault_root = merkle_root(&fault_digests[..]);
    if fault_root != p.candidate.header().faultroot {
        return Err(ConsensusError::InvalidBlock);
    }

    Ok(())
}

pub fn verify_stateless(
    c: &Candidate,
    round_committees: &RoundCommittees,
) -> Result<(), ConsensusError> {
    let iteration = c.header().iteration;
    let generator = round_committees
        .get_generator(iteration)
        .expect("committee to be created before run");
    verify_candidate_msg(c, &generator)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::sync::Arc;

    use super::{ProposalHandler, verify_candidate_msg};
    use dusk_core::signatures::bls::{
        PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
    };
    use dusk_core::transfer::Transaction as ProtocolTransaction;
    use node_data::Serializable;
    use node_data::ledger::Hash;
    use node_data::ledger::{
        Block, Fault, Header, Transaction as LedgerTransaction,
    };
    use node_data::message::payload::Candidate;
    use node_data::message::{
        ConsensusHeader, Message, SignInfo, SignedStepMessage,
    };
    use rand::RngCore;
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    use tokio::sync::Mutex;

    use crate::commons::Database;
    use crate::config::{
        MAX_BLOCK_SIZE, MAX_NUMBER_OF_FAULTS, MAX_NUMBER_OF_TRANSACTIONS,
    };
    use crate::merkle::merkle_root;
    use crate::msg_handler::{MsgHandler, StepOutcome};
    use crate::user::committee::Committee;

    // Keep one unique deterministic seed per test so RNG streams are
    // reproducible and isolated across test cases.
    const SEED_ACCEPT_EXPECTED_GENERATOR: u64 = 1;
    const SEED_ACCEPT_NON_EMPTY_PAYLOAD: u64 = 2;
    const SEED_ACCEPT_PAST_CANDIDATE_COLLECTION: u64 = 3;
    const SEED_REJECT_WRONG_GENERATOR: u64 = 4;
    const SEED_REJECT_INVALID_SIGNATURE: u64 = 5;
    const SEED_REJECT_TXROOT_MISMATCH: u64 = 6;
    const SEED_REJECT_FAULTROOT_MISMATCH: u64 = 7;
    const SEED_REJECT_TOO_MANY_TRANSACTIONS: u64 = 8;
    const SEED_REJECT_TOO_MANY_FAULTS: u64 = 9;
    const SEED_REJECT_OVERSIZED_BLOCK: u64 = 10;
    const SEED_REJECT_PAYLOAD_MUTATION_AFTER_SIGNATURE: u64 = 11;

    #[derive(Default)]
    struct DummyDb {
        stored_candidates: usize,
    }

    // Minimal DB stub that records candidate-store side effects.
    #[async_trait::async_trait]
    impl Database for DummyDb {
        async fn store_candidate_block(&mut self, _b: Block) {
            self.stored_candidates += 1;
        }

        async fn store_validation_result(
            &mut self,
            _ch: &ConsensusHeader,
            _vr: &node_data::message::payload::ValidationResult,
        ) {
        }

        async fn get_last_iter(&self) -> (Hash, u8) {
            ([0u8; 32], 0)
        }

        async fn store_last_iter(&mut self, _data: (Hash, u8)) {}
    }

    // Build and sign a candidate with explicit roots and payload.
    fn build_signed_candidate(
        sk: &BlsSecretKey,
        pk: &node_data::bls::PublicKey,
        txroot: [u8; 32],
        faultroot: [u8; 32],
        txs: Vec<LedgerTransaction>,
        faults: Vec<Fault>,
    ) -> Candidate {
        let mut header = Header::default();
        header.height = 1;
        header.iteration = 0;
        header.prev_block_hash = [7u8; 32];
        header.generator_bls_pubkey = *pk.bytes();
        header.txroot = txroot;
        header.faultroot = faultroot;

        let block = Block::new(header, txs, faults).expect("valid block");
        let mut candidate = Candidate { candidate: block };
        candidate.sign(sk, pk.inner());
        candidate
    }

    // Build a minimally valid signed candidate block
    fn build_candidate(
        sk: &BlsSecretKey,
        pk: &node_data::bls::PublicKey,
        txroot: [u8; 32],
        faultroot: [u8; 32],
    ) -> Candidate {
        build_signed_candidate(sk, pk, txroot, faultroot, vec![], vec![])
    }

    // Build a signed candidate with txroot/faultroot derived from its payload.
    fn build_candidate_with_payload(
        sk: &BlsSecretKey,
        pk: &node_data::bls::PublicKey,
        txs: Vec<LedgerTransaction>,
        faults: Vec<Fault>,
    ) -> Candidate {
        let tx_digests: Vec<_> = txs.iter().map(|tx| tx.digest()).collect();
        let fault_digests: Vec<_> =
            faults.iter().map(|fault| fault.digest()).collect();

        build_signed_candidate(
            sk,
            pk,
            merkle_root(&tx_digests[..]),
            merkle_root(&fault_digests[..]),
            txs,
            faults,
        )
    }

    // Build a compact moonlight transaction suitable for count/size tests.
    fn build_small_transaction(
        sk: &BlsSecretKey,
        nonce: u64,
    ) -> LedgerTransaction {
        let tx = ProtocolTransaction::moonlight(
            sk,
            None,
            0,
            0,
            1,
            1,
            nonce,
            0xFA,
            Option::<Vec<u8>>::None,
        )
        .expect("valid transaction");
        LedgerTransaction::from(tx)
    }

    // Build a minimal decodable fault for fault-count/root verification tests.
    fn build_fault_template(signer: &node_data::bls::PublicKey) -> Fault {
        let mut encoded = vec![0u8];
        let sign_info = SignInfo {
            signer: signer.clone(),
            signature: [9u8; 48].into(),
        };

        for hash_byte in [1u8, 2u8] {
            let header = ConsensusHeader {
                prev_block_hash: [hash_byte; 32],
                round: 1,
                iteration: 0,
            };
            header
                .write(&mut encoded)
                .expect("consensus header should serialize");
            sign_info
                .write(&mut encoded)
                .expect("sign info should serialize");
            encoded.extend_from_slice(&[hash_byte; 32]);
        }

        let mut cursor = Cursor::new(encoded);
        Fault::read(&mut cursor).expect("fault bytes should deserialize")
    }

    // Build a deterministic keypair from the test RNG stream.
    fn generate_random_keypair(
        rng: &mut StdRng,
    ) -> (BlsSecretKey, node_data::bls::PublicKey) {
        let sk = BlsSecretKey::random(rng);
        let pk = node_data::bls::PublicKey::new(BlsPublicKey::from(&sk));
        (sk, pk)
    }

    #[test]
    // Candidate from the expected generator should pass stateless checks.
    fn accept_candidate_from_expected_generator() {
        let mut rng = StdRng::seed_from_u64(SEED_ACCEPT_EXPECTED_GENERATOR);
        let (sk, pk) = generate_random_keypair(&mut rng);
        let empty_root = merkle_root::<[u8; 32]>(&[]);

        let candidate = build_candidate(&sk, &pk, empty_root, empty_root);
        verify_candidate_msg(&candidate, pk.bytes())
            .expect("candidate from expected generator should be valid");
    }

    #[test]
    // Candidate with consistent non-empty tx/fault payload should be accepted.
    fn accept_candidate_with_non_empty_payload() {
        let mut rng = StdRng::seed_from_u64(SEED_ACCEPT_NON_EMPTY_PAYLOAD);
        let (sk, pk) = generate_random_keypair(&mut rng);
        let tx_nonce = rng.next_u64();
        let tx = build_small_transaction(&sk, tx_nonce);
        let fault = build_fault_template(&pk);
        let candidate =
            build_candidate_with_payload(&sk, &pk, vec![tx], vec![fault]);

        let candidate_size = candidate
            .candidate
            .size()
            .expect("candidate size should be known");
        // Ensure the test won't fail due to the block size.
        assert!(candidate_size <= MAX_BLOCK_SIZE);

        verify_candidate_msg(&candidate, pk.bytes())
            .expect("candidate with non-empty payload should be valid");
    }

    #[tokio::test]
    // Past candidate messages should be collected and persisted.
    async fn collect_from_past_stores_candidate_and_returns_ready() {
        let mut rng =
            StdRng::seed_from_u64(SEED_ACCEPT_PAST_CANDIDATE_COLLECTION);
        let (sk, pk) = generate_random_keypair(&mut rng);
        let empty_root = merkle_root::<[u8; 32]>(&[]);
        let candidate = build_candidate(&sk, &pk, empty_root, empty_root);
        let msg: Message = candidate.into();

        let db = Arc::new(Mutex::new(DummyDb::default()));
        let mut handler = ProposalHandler::new(db.clone());
        let committee = Committee::default();

        let outcome = handler
            .collect_from_past(msg, &committee, None)
            .await
            .expect("past candidate should be accepted");
        assert!(matches!(outcome, StepOutcome::Ready(_)));

        let stored = db.lock().await.stored_candidates;
        assert_eq!(stored, 1, "candidate should be stored once");
    }

    #[tokio::test]
    // Past messages with non-candidate payload must be rejected.
    async fn collect_from_past_rejects_invalid_payload() {
        let db = Arc::new(Mutex::new(DummyDb::default()));
        let mut handler = ProposalHandler::new(db);
        let committee = Committee::default();

        let err = match handler
            .collect_from_past(Message::default(), &committee, None)
            .await
        {
            Ok(_) => panic!("expected invalid payload rejection"),
            Err(err) => err,
        };
        assert!(matches!(err, crate::errors::ConsensusError::InvalidMsgType));
    }

    #[test]
    // Candidate generator must match the one extracted by deterministic
    // sortition.
    fn reject_candidate_from_wrong_generator() {
        let mut rng = StdRng::seed_from_u64(SEED_REJECT_WRONG_GENERATOR);
        let (_expected_sk, expected_pk) = generate_random_keypair(&mut rng);
        let (wrong_sk, wrong_pk) = generate_random_keypair(&mut rng);
        let empty_root = merkle_root::<[u8; 32]>(&[]);

        let candidate =
            build_candidate(&wrong_sk, &wrong_pk, empty_root, empty_root);

        let err = verify_candidate_msg(&candidate, expected_pk.bytes())
            .expect_err("expected wrong generator to be rejected");

        assert!(matches!(
            err,
            crate::errors::ConsensusError::NotCommitteeMember
        ));
    }

    #[test]
    // Candidate signature must always verify against signed header payload.
    fn reject_candidate_with_invalid_signature() {
        let mut rng = StdRng::seed_from_u64(SEED_REJECT_INVALID_SIGNATURE);
        let (sk, pk) = generate_random_keypair(&mut rng);
        let empty_root = merkle_root::<[u8; 32]>(&[]);
        let mut candidate = build_candidate(&sk, &pk, empty_root, empty_root);

        let mut sig = *candidate.candidate.header().signature.inner();
        sig[0] ^= 0x01;
        candidate.candidate.set_signature(sig.into());

        let err = verify_candidate_msg(&candidate, pk.bytes())
            .expect_err("expected invalid signature to be rejected");
        assert!(matches!(
            err,
            crate::errors::ConsensusError::InvalidSignature(_)
        ));
    }

    #[test]
    // txroot in the header must match the transactions included in the block.
    fn reject_candidate_with_mismatched_txroot() {
        let mut rng = StdRng::seed_from_u64(SEED_REJECT_TXROOT_MISMATCH);
        let (sk, pk) = generate_random_keypair(&mut rng);

        let candidate =
            build_candidate(&sk, &pk, [1u8; 32], merkle_root::<[u8; 32]>(&[]));
        let err = verify_candidate_msg(&candidate, pk.bytes())
            .expect_err("expected invalid txroot to be rejected");

        assert!(matches!(err, crate::errors::ConsensusError::InvalidBlock));
    }

    #[test]
    // faultroot in the header must match the faults included in the block.
    fn reject_candidate_with_mismatched_faultroot() {
        let mut rng = StdRng::seed_from_u64(SEED_REJECT_FAULTROOT_MISMATCH);
        let (sk, pk) = generate_random_keypair(&mut rng);

        let candidate =
            build_candidate(&sk, &pk, merkle_root::<[u8; 32]>(&[]), [2u8; 32]);
        let err = verify_candidate_msg(&candidate, pk.bytes())
            .expect_err("expected invalid faultroot to be rejected");

        assert!(matches!(err, crate::errors::ConsensusError::InvalidBlock));
    }

    #[test]
    // Candidate with more transactions than the configured maximum is invalid.
    fn reject_candidate_with_too_many_transactions() {
        let mut rng = StdRng::seed_from_u64(SEED_REJECT_TOO_MANY_TRANSACTIONS);
        let (sk, pk) = generate_random_keypair(&mut rng);
        let tx_nonce = rng.next_u64();
        let tx = build_small_transaction(&sk, tx_nonce);
        let txs = vec![tx; MAX_NUMBER_OF_TRANSACTIONS + 1];

        let candidate = build_candidate_with_payload(&sk, &pk, txs, vec![]);
        let candidate_size = candidate
            .candidate
            .size()
            .expect("candidate size should be known");
        // Ensure the test won't fail due to the block size.
        assert!(candidate_size <= MAX_BLOCK_SIZE);

        let err = verify_candidate_msg(&candidate, pk.bytes())
            .expect_err("expected too many transactions to be rejected");

        assert!(matches!(
            err,
            crate::errors::ConsensusError::TooManyTransactions(count)
                if count == MAX_NUMBER_OF_TRANSACTIONS + 1
        ));
    }

    #[test]
    // Candidate with more faults than the configured maximum is invalid.
    fn reject_candidate_with_too_many_faults() {
        let mut rng = StdRng::seed_from_u64(SEED_REJECT_TOO_MANY_FAULTS);
        let (sk, pk) = generate_random_keypair(&mut rng);
        let fault = build_fault_template(&pk);
        let faults = vec![fault; MAX_NUMBER_OF_FAULTS + 1];

        let candidate = build_candidate_with_payload(&sk, &pk, vec![], faults);
        let candidate_size = candidate
            .candidate
            .size()
            .expect("candidate size should be known");
        // Ensure the test won't fail due to the block size.
        assert!(candidate_size <= MAX_BLOCK_SIZE);

        let err = verify_candidate_msg(&candidate, pk.bytes())
            .expect_err("expected too many faults to be rejected");

        assert!(matches!(
            err,
            crate::errors::ConsensusError::TooManyFaults(count)
                if count == MAX_NUMBER_OF_FAULTS + 1
        ));
    }

    #[test]
    // Candidate serialized size must stay within the configured block limit.
    fn reject_oversized_candidate_block() {
        let mut rng = StdRng::seed_from_u64(SEED_REJECT_OVERSIZED_BLOCK);
        let (sk, pk) = generate_random_keypair(&mut rng);
        let tx_nonce = rng.next_u64();
        let tx = build_small_transaction(&sk, tx_nonce);
        let tx_count = MAX_BLOCK_SIZE / tx.size() + 1;
        let txs = vec![tx; tx_count];

        let candidate = build_candidate_with_payload(&sk, &pk, txs, vec![]);
        let candidate_size = candidate
            .candidate
            .size()
            .expect("candidate size should be known");
        assert!(candidate_size > MAX_BLOCK_SIZE);

        let err = verify_candidate_msg(&candidate, pk.bytes())
            .expect_err("expected oversized block to be rejected");
        assert!(matches!(
            err,
            crate::errors::ConsensusError::InvalidBlockSize(size)
                if size > MAX_BLOCK_SIZE
        ));
    }

    #[test]
    // Candidate payload must not be mutable after signature and root commit.
    fn reject_payload_mutation_after_signature() {
        let mut rng =
            StdRng::seed_from_u64(SEED_REJECT_PAYLOAD_MUTATION_AFTER_SIGNATURE);
        let (sk, pk) = generate_random_keypair(&mut rng);
        let tx_nonce = rng.next_u64();
        let tx = build_small_transaction(&sk, tx_nonce);
        let different_tx =
            build_small_transaction(&sk, tx_nonce.wrapping_add(1));
        let mut candidate =
            build_candidate_with_payload(&sk, &pk, vec![tx], vec![]);

        let mut tampered_txs = candidate.candidate.txs().clone();
        tampered_txs.push(different_tx);
        let tampered_header = candidate.candidate.header().clone();
        let tampered_faults = candidate.candidate.faults().clone();
        candidate.candidate =
            Block::new(tampered_header, tampered_txs, tampered_faults)
                .expect("tampered candidate block should still serialize");

        let err = verify_candidate_msg(&candidate, pk.bytes()).expect_err(
            "expected post-signature payload mutation to be rejected",
        );
        assert!(matches!(err, crate::errors::ConsensusError::InvalidBlock));
    }
}
