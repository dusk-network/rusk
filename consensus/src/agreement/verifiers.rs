// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.
use crate::messages;
use crate::messages::payload::StepVotes;
use crate::messages::{Message, Payload};
use crate::user::committee::Committee;
use crate::user::provisioners::Provisioners;
use crate::user::sortition;
use crate::util::cluster::Cluster;
use crate::util::pubkey::PublicKey;
use bytes::{Buf, BufMut, BytesMut};
use dusk_bls12_381_sign::APK;
use dusk_bytes::Serializable;
use tracing::error;

#[derive(Debug)]
pub enum Error {
    VoteSetTooSmall,
    VerificationFailed,
    EmptyApk,
    InvalidType,
}

/// verify_agreement performs all three-steps verification of an agreement message. It is intended to be used in a context of tokio::spawn as per that it tries to yield before any CPU-bound operation.
pub async fn verify_agreement(
    msg: Message,
    provisioners: &mut Provisioners,
    seed: [u8; 32],
) -> Result<(), Error> {
    match msg.payload {
        Payload::Agreement(payload) => {
            if let Err(e) = verify_whole(&msg.header, payload.signature) {
                error!("{}", e);
                return Err(Error::VerificationFailed);
            }

            // Verify 1th_reduction step_votes
            verify_step_votes(payload.votes_per_step.0, provisioners, seed, &msg.header, 0).await?;

            // Verify 2th_reduction step_votes
            verify_step_votes(payload.votes_per_step.1, provisioners, seed, &msg.header, 1).await?;

            // Verification done
            Ok(())
        }
        _ => Err(Error::InvalidType),
    }
}

async fn verify_step_votes(
    sv: StepVotes,
    provisioners: &mut Provisioners,
    seed: [u8; 32],
    hdr: &messages::Header,
    step_offset: u8,
) -> Result<(), Error> {
    tokio::task::yield_now().await;

    let step = hdr.step - 1 + step_offset;
    let c = Committee::new(
        PublicKey::default(),
        provisioners,
        sortition::Config::new(seed, hdr.round, step, 64),
    );

    let sub_committee = c.intersect(sv.bitset);

    if c.total_occurrences(&sub_committee) < c.quorum() {
        return Err(Error::VoteSetTooSmall);
    }

    unsafe {
        // aggregate public keys
        let apk = aggregate_pks(&provisioners, sub_committee)?;

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

unsafe fn aggregate_pks(
    provisioners: &Provisioners,
    subcomittee: Cluster<PublicKey>,
) -> Result<dusk_bls12_381_sign::APK, Error> {
    let mut pks = vec![];

    let _ = subcomittee.into_iter().map(|member| {
        if let Some(m) = provisioners.get_member(&member.0) {
            pks.push(dusk_bls12_381_sign::PublicKey::from_slice_unchecked(
                &m.get_raw_key(),
            ));
        } else {
            debug_assert!(false, "raw public key not found");
        }
    });

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

fn marshal_signable_vote(round: u64, step: u8, block_hash: [u8; 32]) -> BytesMut {
    let mut msg = BytesMut::with_capacity(block_hash.len() + 8 + 1);
    msg.put_u64_le(round);
    msg.put_u8(step);
    msg.put(&block_hash[..]);

    msg
}
