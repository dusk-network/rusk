// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Conformance of the Dusk typed-data BLS vector corpus against this
//! implementation.
//!
//! The corpus is the interoperability contract for `dusk_signTypedData`
//! (dusk-network/wallet#22). It is shared by three implementations: this one,
//! the Connect SDK verifier, and the wallet extension signer. The JavaScript
//! sides check themselves against it; this test checks the corpus against the
//! implementation that defines the protocol, so the two cannot agree with each
//! other and disagree with the chain.
//!
//! The specific failure it guards against is quiet. A JavaScript implementation
//! that leaves its library's default IETF ciphersuite
//! (`BLS_SIG_BLS12381G1_XMD:SHA-256_SSWU_RO_NUL_`) in place instead of Dusk's
//! `BLS_SIG_BLS12381G1_XMD:SHA-256_DUSK_V2` is entirely self-consistent: it
//! verifies its own signatures, every round-trip test passes, and every
//! signature it produces is rejected on chain. Only fixed expected bytes,
//! checked here, catch that.
//!
//! Vectors are vendored from dusk-network/typed-data; see `tests/vectors/SOURCE`
//! for the commit they were taken from. They are not generated here.

use std::fs;
use std::path::{Path, PathBuf};

use dusk_bytes::Serializable;
use dusk_core::signatures::bls::{PublicKey, SecretKey};
use dusk_wallet_core::keys::derive_bls_sk;

/// Typed-data signature domain tag, spec section 12.1.
///
/// 23 bytes including the trailing NUL. Signatures cover `SIG_TAG || digest`,
/// never the bare 32-byte digest, so that the typed-data signed-message space
/// is structurally disjoint from every other 32-byte message this key signs.
const SIG_TAG: &[u8] = b"DUSK_TYPED_DATA_SIG_V1\0";

fn vector_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/vectors/bls-v1")
}

fn decode_hex(value: &str) -> Vec<u8> {
    let body = value.strip_prefix("0x").unwrap_or(value);
    hex::decode(body).expect("vector field is valid hex")
}

struct Vector {
    name: String,
    seed: Vec<u8>,
    profile_index: u8,
    digest: Vec<u8>,
    dst: String,
    bls_version: String,
    sig_tag: String,
    expected_sk_le: Vec<u8>,
    expected_pk: Vec<u8>,
    expected_signed_message: Vec<u8>,
    expected_signature: Vec<u8>,
}

fn load_vectors() -> Vec<Vector> {
    let dir = vector_dir();
    let mut entries: Vec<_> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .map(|entry| entry.expect("readable dir entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    entries.sort();

    entries
        .iter()
        .map(|path| {
            let raw = fs::read_to_string(path).expect("vector is readable");
            let json: serde_json::Value =
                serde_json::from_str(&raw).expect("vector is valid JSON");

            let profile_index = json["input"]["profileIndex"]
                .as_u64()
                .expect("profileIndex is a number");
            assert!(
                profile_index <= u64::from(u8::MAX),
                "profileIndex {profile_index} does not fit the u8 index \
                 derive_bls_sk takes"
            );

            let field = |group: &str, key: &str| -> String {
                json[group][key]
                    .as_str()
                    .unwrap_or_else(|| {
                        panic!("{}: missing {group}.{key}", path.display())
                    })
                    .to_owned()
            };

            Vector {
                name: path
                    .file_stem()
                    .expect("vector has a file stem")
                    .to_string_lossy()
                    .into_owned(),
                seed: decode_hex(&field("input", "seedHex")),
                profile_index: profile_index as u8,
                digest: decode_hex(&field("input", "digestHex")),
                dst: field("params", "dst"),
                bls_version: field("params", "blsVersion"),
                sig_tag: field("params", "sigTag"),
                expected_sk_le: decode_hex(&field(
                    "expected",
                    "secretKeyLeHex",
                )),
                expected_pk: decode_hex(&field("expected", "publicKeyG2Hex")),
                expected_signed_message: decode_hex(&field(
                    "expected",
                    "signedMessageHex",
                )),
                expected_signature: decode_hex(&field(
                    "expected",
                    "signatureG1Hex",
                )),
            }
        })
        .collect()
}

#[test]
fn corpus_is_present() {
    // A vendoring or path mistake that empties the directory must fail loudly
    // rather than turning every assertion below into a silent no-op.
    assert!(
        !load_vectors().is_empty(),
        "no vectors found in {} - is the corpus vendored?",
        vector_dir().display()
    );
}

#[test]
fn corpus_pins_the_v2_parameters() {
    for vector in load_vectors() {
        assert_eq!(
            vector.dst, "BLS_SIG_BLS12381G1_XMD:SHA-256_DUSK_V2",
            "{}: DST is not the Dusk V2 hash-to-curve tag \
             (bls12_381-bls src/hash.rs H0_DST)",
            vector.name
        );
        assert_eq!(
            vector.bls_version, "V2",
            "{}: typed data is V2-only, independent of chain height",
            vector.name
        );
        assert_eq!(
            vector.sig_tag.as_bytes(),
            SIG_TAG,
            "{}: signature domain tag does not match spec section 12.1",
            vector.name
        );
    }
}

#[test]
fn derivation_matches_wallet_core() {
    for vector in load_vectors() {
        let seed: [u8; 64] =
            vector.seed.as_slice().try_into().unwrap_or_else(|_| {
                panic!("{}: seed must be 64 bytes", vector.name)
            });

        let sk = derive_bls_sk(&seed, vector.profile_index);

        assert_eq!(
            sk.to_bytes().as_slice(),
            vector.expected_sk_le.as_slice(),
            "{}: derive_bls_sk disagrees with the corpus. The expected value is \
             little-endian BlsScalar::to_bytes - if a JS twin produced the \
             mismatch, fix the twin, not this vector.",
            vector.name
        );
    }
}

#[test]
fn public_keys_match_the_corpus() {
    for vector in load_vectors() {
        let seed: [u8; 64] =
            vector.seed.as_slice().try_into().expect("64-byte seed");
        let sk = derive_bls_sk(&seed, vector.profile_index);
        let pk = PublicKey::from(&sk);

        assert_eq!(
            pk.to_bytes().as_slice(),
            vector.expected_pk.as_slice(),
            "{}: public key mismatch - Dusk is the min-sig variant, so this is \
             96 bytes of compressed G2",
            vector.name
        );
    }
}

#[test]
fn signed_message_is_tag_then_digest() {
    for vector in load_vectors() {
        assert_eq!(
            vector.digest.len(),
            32,
            "{}: digest must be 32 bytes",
            vector.name
        );
        assert_eq!(
            vector.expected_signed_message.len(),
            SIG_TAG.len() + 32,
            "{}: signed message must be SIG_TAG || digest (55 bytes)",
            vector.name
        );

        let (tag, digest) =
            vector.expected_signed_message.split_at(SIG_TAG.len());
        assert_eq!(tag, SIG_TAG, "{}: wrong signature domain tag", vector.name);
        assert_eq!(
            digest,
            vector.digest.as_slice(),
            "{}: signed message does not end with the vector's digest",
            vector.name
        );
    }
}

#[test]
fn signatures_match_the_corpus_byte_for_byte() {
    for vector in load_vectors() {
        let seed: [u8; 64] =
            vector.seed.as_slice().try_into().expect("64-byte seed");
        let sk = derive_bls_sk(&seed, vector.profile_index);

        // `SecretKey::sign` is the secure V2 path, the same one
        // `dusk_core::signatures::bls::sign(.., BlsVersion::V2)` dispatches to.
        let signature = sk.sign(&vector.expected_signed_message);

        // BLS is deterministic, so this is an exact byte comparison rather than
        // a verification check. A verification check would still pass under a
        // wrong DST, which is the failure this corpus exists to catch.
        assert_eq!(
            signature.to_bytes().as_slice(),
            vector.expected_signature.as_slice(),
            "{}: signature mismatch - a JS twin using its library's default \
             IETF ciphersuite instead of the Dusk V2 DST produces exactly this \
             failure while its own round-trip tests still pass",
            vector.name
        );
    }
}

#[test]
fn corpus_signatures_verify() {
    for vector in load_vectors() {
        let seed: [u8; 64] =
            vector.seed.as_slice().try_into().expect("64-byte seed");
        let sk = derive_bls_sk(&seed, vector.profile_index);
        let pk = PublicKey::from(&sk);

        let signature =
            bls_signature_from_bytes(&vector.expected_signature, &vector.name);

        assert!(
            pk.verify(&signature, &vector.expected_signed_message)
                .is_ok(),
            "{}: corpus signature does not verify under the derived key",
            vector.name
        );

        // The bare digest must NOT verify: the tag is what keeps the
        // typed-data signing space disjoint from raw-digest paths.
        assert!(
            pk.verify(&signature, &vector.digest).is_err(),
            "{}: corpus signature verifies over the bare digest - the domain \
             tag is not providing separation",
            vector.name
        );
    }
}

fn bls_signature_from_bytes(
    bytes: &[u8],
    name: &str,
) -> dusk_core::signatures::bls::Signature {
    let array: [u8; 48] = bytes.try_into().unwrap_or_else(|_| {
        panic!("{name}: signature must be 48 bytes (compressed G1)")
    });
    Serializable::<48>::from_bytes(&array)
        .unwrap_or_else(|_| panic!("{name}: signature is not a valid G1 point"))
}

/// Guards the assumption every other test in this file rests on.
///
/// `SecretKey::random` draws its scalar from the RNG, so `derive_bls_sk` is
/// only reproducible because `rng_with_index` is seeded deterministically. If
/// that ever stops being true, the corpus does not merely fail - it becomes
/// meaningless, and this test says so directly.
#[test]
fn derivation_is_deterministic() {
    let seed = [7u8; 64];
    let first: SecretKey = derive_bls_sk(&seed, 3);
    let second: SecretKey = derive_bls_sk(&seed, 3);

    assert_eq!(
        first.to_bytes(),
        second.to_bytes(),
        "derive_bls_sk is not deterministic for a fixed (seed, index)"
    );

    let other = derive_bls_sk(&seed, 4);
    assert_ne!(
        first.to_bytes(),
        other.to_bytes(),
        "derive_bls_sk ignores the profile index"
    );
}
