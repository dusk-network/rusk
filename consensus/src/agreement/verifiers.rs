// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::commons::marshal_signable_vote;
use crate::messages;
use crate::messages::payload::StepVotes;
use crate::messages::{Message, Payload};
use crate::user::committee::CommitteeSet;
use crate::user::sortition;
use crate::util::cluster::Cluster;
use crate::util::pubkey::PublicKey;
use bytes::{Buf, BufMut, BytesMut};
use dusk_bls12_381_sign::APK;
use dusk_bytes::Serializable;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::error;

#[derive(Debug)]
pub enum Error {
    VoteSetTooSmall,
    VerificationFailed,
    EmptyApk,
    InvalidType,
    NotCommitteeMember,
}

/// verify_agreement performs all three-steps verification of an agreement message. It is intended to be used in a context of tokio::spawn as per that it tries to yield before any CPU-bound operation.
pub async fn verify_agreement(
    msg: Message,
    committees_set: Arc<Mutex<CommitteeSet>>,
    seed: [u8; 32],
) -> Result<(), Error> {
    match msg.payload {
        Payload::Agreement(payload) => {
            if let Err(e) = verify_whole(&msg.header, payload.signature) {
                error!("{}", e);
                return Err(Error::VerificationFailed);
            }

            // Verify 1th_reduction step_votes
            verify_step_votes(
                payload.votes_per_step.0,
                committees_set.clone(),
                seed,
                &msg.header,
                0,
                false,
            )
            .await?;

            // Verify 2th_reduction step_votes
            verify_step_votes(
                payload.votes_per_step.1,
                committees_set,
                seed,
                &msg.header,
                1,
                true,
            )
            .await?;

            // Verification done
            Ok(())
        }
        _ => Err(Error::InvalidType),
    }
}

async fn verify_step_votes(
    sv: StepVotes,
    committees_set: Arc<Mutex<CommitteeSet>>,
    seed: [u8; 32],
    hdr: &messages::Header,
    step_offset: u8,
    enable_membership_check: bool,
) -> Result<(), Error> {
    tokio::task::yield_now().await;

    let step = hdr.step - 1 + step_offset;
    let cfg = sortition::Config::new(seed, hdr.round, step, 64);

    if enable_membership_check && !committees_set.lock().await.is_member(hdr.pubkey_bls, cfg) {
        return Err(Error::NotCommitteeMember);
    }

    let sub_committee = {
        // Scoped guard to fetch committee data quickly
        let mut guard = committees_set.lock().await;

        let sub_committee = guard.intersect(sv.bitset, cfg);
        let target_quorum = guard.quorum(cfg);

        if guard.total_occurrences(&sub_committee, cfg) < target_quorum {
            return Err(Error::VoteSetTooSmall);
        }

        Ok(sub_committee)
    }?;

    unsafe {
        // aggregate public keys
        let apk = aggregate_pks(committees_set.clone(), sub_committee).await?;

        tokio::task::yield_now().await;

        // verify signatures
        if let Err(e) = verify_signatures(hdr.round, step, hdr.block_hash, apk, sv.signature) {
            error!("verify signatures fails with err: {}", e);
            return Err(Error::VerificationFailed);
        }
    }

    // Verification done
    Ok(())
}

async unsafe fn aggregate_pks(
    committees_set: Arc<Mutex<CommitteeSet>>,
    subcomittee: Cluster<PublicKey>,
) -> Result<dusk_bls12_381_sign::APK, Error> {
    let pks = {
        let mut pks = vec![];

        let guard = committees_set.lock().await;
        let provisioners = guard.get_provisioners();

        for m in subcomittee.into_iter() {
            if let Some(m) = provisioners.get_member(&m.0) {
                pks.push(dusk_bls12_381_sign::PublicKey::from_slice_unchecked(
                    &m.get_raw_key(),
                ));
            } else {
                debug_assert!(false, "raw public key not found");
            }
        }

        pks
    };

    if pks.is_empty() {
        return Err(Error::EmptyApk);
    }

    let mut apk = APK::from(pks.get_unchecked(0));
    if pks.len() > 1 {
        apk.aggregate(&pks[1..]);
    }

    Ok(apk)
}

fn verify_signatures(
    round: u64,
    step: u8,
    block_hash: [u8; 32],
    apk: dusk_bls12_381_sign::APK,
    signature: [u8; 48],
) -> Result<(), dusk_bls12_381_sign::Error> {
    // Compile message to verify

    let sig = dusk_bls12_381_sign::Signature::from_bytes(&signature.into())?;
    apk.verify(&sig, marshal_signable_vote(round, step, block_hash).bytes())
}

fn verify_whole(
    hdr: &messages::Header,
    signature: [u8; 48],
) -> Result<(), dusk_bls12_381_sign::Error> {
    let sig = dusk_bls12_381_sign::Signature::from_bytes(&signature.into())?;

    APK::from(&hdr.pubkey_bls.to_bls_pk()).verify(
        &sig,
        marshal_signable_vote(hdr.round, hdr.step, hdr.block_hash).bytes(),
    )
}
