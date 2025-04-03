// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! This module implements the EIP2333 derivation scheme for BLS keys
//! as defined at <https://eips.ethereum.org/EIPS/eip-2333>
//!
//! Specification
//!
//! Keys are defined in terms of a tree structure where a key is determined by
//! the tree’s seed and a tree path. The specification can be broken into two
//! sub-components: generating the master key, and constructing a child key from
//! its parent. The master key is used as the root of the tree and then the tree
//! is built in layers on top of this root.
//!
//! Tree Structure
//!
//! The key tree is defined purely through the relationship between a child-node
//! and its ancestors. Starting with the root of the tree, the master key, a
//! child node can be derived by knowing the parent’s private key and the index
//! of the child. The tree is broken up into depths which are indicated by / and
//! the master node is described as m. The first child of the master node is
//! therefore described as m / 0 and m / 0’s siblings are m / i for all 0 <= i <
//! 2^32.
//!
//! ```text
//!       [m / 0] - [m / 0 / 0]
//!      /        \
//!     /           [m / 0 / 1]
//! [m] - [m / 1]
//!     \
//!      ...
//!       [m / i]
//! ```
//!
//! Derivation
//!
//! Every key generated via the key derivation process derives a child key via a
//! set of intermediate Lamport keys. The idea behind the Lamport keys is to
//! provide a post-quantum backup in case BLS12-381 is no longer deemed secure.
//! At a high level, the key derivation process works by using the parent node’s
//! privkey as an entropy source for the Lamport private keys which are then
//! hashed together into a compressed Lamport public key, this public key is
//! then hashed into BLS12-381’s private key group.
//!
//! EIP2333 procedures:
//!  - `IKM_to_lamport_SK`
//!  - `parent_SK_to_lamport_PK`
//!  - `HKDF_mod_r`
//!  - `derive_child_SK`
//!  - `derive_master_SK`
//!
//! External definitions:
//!  - `I2OSP`: defined in RFC3447 (Big endian decoding)
//!  - `OS2IP`: defined in RFC3447 (Big endian encoding)
//!  - `HKDF-Extract`: defined in RFC5869, instantiated with SHA256
//!  - `HKDF-Expand`: defined in RFC5869, instantiated with SHA256

use crate::Seed;
use dusk_core::BlsScalar;

use hkdf::Hkdf;
use sha2::{Digest, Sha256};

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::{format, vec};

const SHA256_DIGEST_SIZE: usize = 32;
const HKDF_DIGESTS: usize = 255;
const HKDF_OUTPUT_SIZE: usize = SHA256_DIGEST_SIZE * HKDF_DIGESTS;

/// HKDF
///
/// Derives output keying material (OKM) using the HMAC-based Extract-and-Expand
/// Key Derivation Function (HKDF) with SHA-256 as the underlying hash function.
///
/// This function performs two main steps:
/// 1. **HKDF-Extract:** Combines the provided `salt` and input keying material
///    (IKM) to compute a pseudorandom key (PRK).
/// 2. **HKDF-Expand:** Uses the PRK along with the application-specific `info`
///    to derive OKM.
///
///
/// # Arguments
/// * `salt` - The salt for the HKDF-Extract phase.
/// * `ikm`  - The input keying material for the HKDF-Extract phase.
/// * `info` - Application-specific information used in the expansion phase.
/// * `okm`  - A mutable byte slice to fill with the derived OKM. Its length
///   determines the OKM size.
///
/// # Panics
/// Panics if the HKDF-Expand operation fails due to an invalid length of the
/// `okm` slice (`okm.length() > 255 * size_of(usize)`).
#[allow(clippy::similar_names)]
fn hkdf(salt: &[u8], ikm: &[u8], info: &[u8], okm: &mut [u8]) {
    // PRK = HKDF-Extract(salt, IKM)
    let prk = Hkdf::<Sha256>::new(Some(salt), ikm);

    // OKM = HKDF-Expand(PRK, info , L)
    //
    // NOTE: L = okm.length()
    prk.expand(info, okm)
        .expect("okm size to be a valid length HKDF-Expand");
}

/// Derives a set of digests for a Lamport secret key from the given input
/// keying material (IKM) using HKDF.
///
/// # EIP2333 Specification
///
/// Inputs
///  - `IKM`, a secret octet string
///  - `salt`, an octet string
///
/// Outputs
///  - `lamport_SK`, an array of 255 32-octet strings
///
/// Definitions
///  - `K` = 32 is the digest size (in octets) of the hash function (SHA256)
///  - `L` = K * 255 is the HKDF output size (in octets)
///  - `bytes_split` is a function takes in an octet string and splits it into
///    K-byte chunks which are returned as an array
///
/// Procedure
///
///  0. `PRK = HKDF-Extract(salt, IKM)`
///  1. `OKM = HKDF-Expand(PRK, "" , L)`
///  2. `lamport_SK = bytes_split(OKM, K)`
///  3. `return lamport_SK`
///
///
/// # Arguments
/// * `ikm`        - The initial keying material for HKDF.
/// * `salt`       - The salt for HKDF.
/// * `lamport_sk` - A container for the resulting lamport SK
///
/// # Panics
/// Panics if the HKDF expansion fails due to an invalid output length
/// (`okm.length() > 255 * size_of(usize)`).
#[allow(clippy::similar_names)]
fn ikm_to_lamport_sk(
    ikm: &[u8],
    salt: &[u8],
    lamport_sk: &mut [[u8; SHA256_DIGEST_SIZE]; HKDF_DIGESTS],
) {
    let mut okm = [0u8; HKDF_OUTPUT_SIZE];

    // PRK = HKDF-Extract(salt, IKM)
    // OKM = HKDF-Expand(PRK, "" , L)
    hkdf(salt, ikm, b"", &mut okm);

    // lamport_SK = bytes_split(OKM, K)
    for r in 0..HKDF_DIGESTS {
        lamport_sk[r].copy_from_slice(
            &okm[r * SHA256_DIGEST_SIZE..(r + 1) * SHA256_DIGEST_SIZE],
        );
    }
}

/// Derives a Lamport public key from a parent secret key and an index.
///
/// # EIP2333 Specification:
///
/// Inputs
///  - `parent_SK`: the BLS Secret Key of the parent node
///  - `index`: the index of the child node, an integer 0 <= index < 2^32
///
/// Outputs
///  - `lamport_PK`: the compressed lamport PK, a 32 octet string
///
/// Definitions
///  - `flip_bits`: a function that returns the bitwise negation of its input
///
/// Procedure
///
///  0. `salt = I2OSP(index, 4)`
///  1. `IKM = I2OSP(parent_SK, 32)`
///  2. `lamport_0 = IKM_to_lamport_SK(IKM, salt)`
///  3. `not_IKM = flip_bits(IKM)`
///  4. `lamport_1 = IKM_to_lamport_SK(not_IKM, salt)`
///  5. `lamport_PK = ""`
///  6. `for i  in 1, .., 255 lamport_PK = lamport_PK | SHA256(lamport_0[i])`
///  7. `for i  in 1, .., 255 lamport_PK = lamport_PK | SHA256(lamport_1[i])`
///  8. `compressed_lamport_PK = SHA256(lamport_PK)`
///  9. `return compressed_lamport_PK`
///
///
/// # Arguments
/// * `parent_sk` - The parent secret key.
/// * `index`     - The child index.
///
/// # Returns
/// The compressed Lamport public key as derived from the parent secret key.
///
/// # Panics
/// Panics if any of the `ikm_to_lamport_sk` fails.
fn parent_sk_to_lamport_pk(parent_sk: &BlsScalar, index: u32) -> Vec<u8> {
    // salt = I2OSP(index, 4)
    let salt = index.to_be_bytes();

    // IKM = I2OSP(parent_SK, 32)
    let ikm = parent_sk.to_be_bytes();

    // lamport_0 = IKM_to_lamport_SK(IKM, salt)
    let mut lamport_0 = [[0u8; SHA256_DIGEST_SIZE]; HKDF_DIGESTS];
    ikm_to_lamport_sk(ikm.as_slice(), salt.as_slice(), &mut lamport_0);

    // not_IKM = flip_bits(IKM)
    let not_ikm = ikm.map(|byte| !byte);

    // lamport_1 = IKM_to_lamport_SK(not_IKM, salt)
    let mut lamport_1 = [[0u8; SHA256_DIGEST_SIZE]; HKDF_DIGESTS];
    ikm_to_lamport_sk(not_ikm.as_slice(), salt.as_slice(), &mut lamport_1);

    // Combine `lamport_0` and `lamport_1` arrays
    let mut lamport_combined = [[0u8; SHA256_DIGEST_SIZE]; HKDF_DIGESTS * 2];
    lamport_combined[..HKDF_DIGESTS]
        .clone_from_slice(&lamport_0[..HKDF_DIGESTS]);
    lamport_combined[HKDF_DIGESTS..HKDF_DIGESTS * 2]
        .clone_from_slice(&lamport_1[..HKDF_DIGESTS]);

    // for i  in 1, .., 255
    //    lamport_PK = lamport_PK | SHA256(lamport_0[i])
    // for i  in 1, .., 255
    //    lamport_PK = lamport_PK | SHA256(lamport_1[i])
    let mut lamport_pk = [0u8; HKDF_OUTPUT_SIZE * 2];
    for i in 0..HKDF_DIGESTS * 2 {
        let sha_slice = &Sha256::digest(lamport_combined[i]);
        lamport_pk[i * SHA256_DIGEST_SIZE..(i + 1) * SHA256_DIGEST_SIZE]
            .clone_from_slice(sha_slice);
    }

    // compressed_lamport_PK = SHA256(lamport_PK)
    Sha256::digest(lamport_pk).to_vec()
}

/// Derives a BLS scalar using HKDF extraction and expansion.
///
/// # EIP2333 Specification:
///
/// Inputs
///  - `IKM`, a secret octet string >= 256 bits in length
///  - `key_info`, an optional octet string (default="", the empty string)
///
/// Output
///  - `SK`, the corresponding secret key, an integer 0 <= SK < r.
///
/// Definitions
///  - `L`: integer given by `ceil((3 * ceil(log2(r)))` / 16).(L=48)
///  - `r`: the order of the BLS 12-381 curve defined in the v4 draft IETF BLS
///    signature scheme standard
///
/// Procedure
///
///  1. `salt = "BLS-SIG-KEYGEN-SALT-"`
///  2. `SK = 0`
///  3. `while SK == 0:`
///  4. `salt = H(salt)`
///  5. `PRK = HKDF-Extract(salt, IKM || I2OSP(0, 1))`
///  6. `OKM = HKDF-Expand(PRK, key_info || I2OSP(L, 2), L)`
///  7. `SK = OS2IP(OKM) mod r`
///  8. `return SK`
///
///
/// # Arguments
/// * `ikm`      - The initial keying material for HKDF.
/// * `key_info` - The info for HKDF.
///
/// # Returns
/// The derived secret key.
///
/// # Panics
/// Panics if HKDF extraction/expansion fail.
#[allow(clippy::similar_names)]
fn hkdf_mod_r(ikm: &[u8], key_info: &[u8]) -> BlsScalar {
    const L: usize = 48;

    // IKM || I2OSP(0, 1)
    let ikm_combined = [ikm, &[0u8]].concat();
    // key_info || I2OSP(L, 2)
    let key_info_combined = [
        key_info,
        &[0u8, u8::try_from(L).expect("L should be castable to u8")],
    ]
    .concat();

    // HKDF output size L (=48)
    let mut okm: [u8; L] = [0u8; L];

    // SK = 0
    let mut sk = BlsScalar::zero();

    // salt = "BLS-SIG-KEYGEN-SALT-"
    // salt = H(salt)
    let mut salt = Sha256::digest(b"BLS-SIG-KEYGEN-SALT-");

    while sk.is_zero().into() {
        // PRK = HKDF-Extract(salt, IKM || I2OSP(0, 1))
        // OKM = HKDF-Expand(PRK, key_info || I2OSP(L, 2), L)
        hkdf(&salt, ikm_combined.as_ref(), &key_info_combined, &mut okm);

        // Convert okm to a 64-byte little-endian value
        let mut okm_le_64 = [0u8; 64];
        okm.reverse();
        okm_le_64[..L].copy_from_slice(&okm);

        // SK = OS2IP(OKM) mod r
        sk = BlsScalar::from_bytes_wide(&okm_le_64);

        // salt = H(salt)
        // Since this is needed for the next iteration, we only compute the
        // digest if such iteration is going to be executed
        if sk.is_zero().into() {
            salt = Sha256::digest(salt);
        }
    }

    sk
}

/// Derives the child secret key from the parent secret key and an index.
///
/// # EIP2333 Specification
///
/// Inputs
///  - `parent_SK`: the secret key of the parent node, a big-endian encoded
///    integer
///  - `index`: the index of the child node, an integer 0 <= index < 2^32
///
/// Outputs
///  - `child_SK`: the secret key of the child node, a big-endian encoded
///    integer
///
/// Procedure
///
///  0. `compressed_lamport_PK = parent_SK_to_lamport_PK(parent_SK, index)`
///  1. `SK = HKDF_mod_r(compressed_lamport_PK)`
///  2. `return SK`
///
///
/// # Arguments
/// * `parent_sk` - The parent secret key.
/// * `index`     - The child index.
///
/// # Returns
/// The derived child secret key.
///
/// # Panics
/// Panics if `parent_sk_to_lamport_pk` or `hkdf_mod_r` fail.
#[must_use]
fn derive_child_sk(parent_sk: &BlsScalar, index: u32) -> BlsScalar {
    // NOTE: `parent_sk` is in little-endian encoding but it's converted to
    // big-endian encoding by `parent_sk_to_lamport_pk`

    // compressed_lamport_PK = parent_SK_to_lamport_PK(parent_SK, index)
    let lamport_pk = parent_sk_to_lamport_pk(parent_sk, index);

    // SK = HKDF_mod_r(compressed_lamport_PK)
    hkdf_mod_r(lamport_pk.as_ref(), b"")
}

/// Derives the master BLS secret key from a given seed using an HKDF-based
/// approach. The seed should be derived from a BIP39 mnemonic with the
/// associated `mnemonic_to_seed` method be used.
///
/// # EIP2333 Specification:
///
/// Inputs
///  - `seed`: the source entropy for the entire tree, a octet string >= 256
///    bits in length
///
/// Outputs
///  - `SK`, the secret key of master node within the tree, a big-endian encoded
///    integer
///
/// Procedure
///
///  0. `SK = HKDF_mod_r(seed)`
///  1. `return SK`
///
///
/// # Arguments
/// * `seed` - The seed for the master key derivation.
///
/// # Returns
/// Returns the derived master secret key if the derivation succeeds, or an
/// error otherwise.
///
/// # Errors
/// This function returns an error if the `seed` is less than 32-byte long
///
/// # Panics
/// Panics if `hkdf_mod_r`fails.
fn derive_master_sk(seed: &[u8]) -> Result<BlsScalar, String> {
    if seed.len() < 32 {
        return Err(
            "seed must be greater than or equal to 32 bytes".to_string()
        );
    }

    // SK = HKDF_mod_r(seed)
    Ok(hkdf_mod_r(seed, b""))
}

/// Parses a derivation path string and returns a vector of child index values.
///
/// The input string must start with `"m"` followed by one or more numeric
/// segments separated by `/`. Each numeric segment is parsed as a base-10
/// `u32`. If the format is incorrect or a segment cannot be parsed, an error is
/// returned.
///
/// # Arguments
/// * `path_str` - A string slice representing the derivation path, e.g.,
///   `"m/44/0/0"`.
///
/// # Returns
/// Returns a vector of `u32` child indexes if parsing is successful, or an
/// error if parsing fails.
///
/// # Errors
/// This function returns an error if the first node in the path is not `m`, one
/// of the child indexes is not an integer between 0 and 2^32, or the path
/// contains no child node
fn get_path_indexes(path_str: &str) -> Result<Vec<u32>, String> {
    let mut path: Vec<&str> = path_str.split('/').collect();
    let m = path.remove(0);
    if m != "m" {
        return Err(format!("First node must be m, got {m}"));
    }

    let mut ret: Vec<u32> = vec![];
    for index in path {
        match index.parse::<u32>() {
            Ok(v) => ret.push(v),
            Err(_) => return Err("could not parse node: {index}".to_string()),
        }
    }

    if ret.is_empty() {
        return Err("Path contains no child index".to_string());
    }

    Ok(ret)
}

/// Derives a BLS secret key using the EIP-2333 specification from a given seed
/// and derivation path.
///
/// This function first derives the master secret key from the provided seed. It
/// then parses the derivation path to extract the sequence of child indexes and
/// iteratively derives the corresponding child keys. The final derived BLS
/// secret key is returned.
///
/// # Arguments
/// * `seed` - A reference to a `Seed` used to derive the master secret key.
/// * `path` - A string slice representing the derivation path (e.g.,
///   `"m/0/1/2"`).
///
/// # Returns
/// The derived BLS secret key if derivation succeeds, or an error otherwise.
///
/// # Errors
/// This function returns an error if it fails to generate the master secret key
/// or if the derivation path is improperly formatted or contains invalid node
/// indexes.
///
/// # Panics
/// This function panics if any of the called functions panic.
pub fn derive_bls_sk(seed: &Seed, path: &str) -> Result<BlsScalar, String> {
    // Derive master key
    let master_key: BlsScalar = derive_master_sk(seed)?;

    // Parse nodes indexes from path
    let path_indexes: Vec<u32> = get_path_indexes(path)?;
    let mut node_sk = master_key;

    // Derive path keys
    for index in &path_indexes {
        node_sk = derive_child_sk(&node_sk, *index);
    }

    Ok(node_sk)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bip39::{Language, Mnemonic, Seed};
    use hex::decode;
    use num_bigint::BigUint;

    struct TestCase {
        seed: &'static str,
        master_sk: &'static str,
        child_index: &'static str,
        child_sk: &'static str,
    }

    // Tests derivation from a given seed to the master key, and from
    // the master key to a child key with a given index.
    #[test]
    fn test_child_derivation() {
        // All test cases are taken from the EIP2333 specification
        let test_cases = vec!(
            TestCase{
                seed : "c55257c360c07c72029aebc1b53c05ed0362ada38ead3e3e9efa3708e53495531f09a6987599d18264c1e1c92f2cf141630c7a3c4ab7c81b2f001698e7463b04",
                master_sk : "6083874454709270928345386274498605044986640685124978867557563392430687146096",
                child_index : "0",
                child_sk : "20397789859736650942317412262472558107875392172444076792671091975210932703118",
            },
            TestCase{
                seed: "0099FF991111002299DD7744EE3355BBDD8844115566CC55663355668888CC00",
                master_sk: "27580842291869792442942448775674722299803720648445448686099262467207037398656",
                child_index: "4294967295",
                child_sk: "29358610794459428860402234341874281240803786294062035874021252734817515685787",
            },
            TestCase{
                seed: "3141592653589793238462643383279502884197169399375105820974944592",
                master_sk: "29757020647961307431480504535336562678282505419141012933316116377660817309383",
                child_index: "3141592653",
                child_sk: "25457201688850691947727629385191704516744796114925897962676248250929345014287",
            },
            TestCase{
                seed: "d4e56740f876aef8c010b86a40d5f56745a118d0906a34e69aec8c0db1cb8fa3",
                master_sk: "19022158461524446591288038168518313374041767046816487870552872741050760015818",
                child_index: "42",
                child_sk: "31372231650479070279774297061823572166496564838472787488249775572789064611981",
            }
        );

        for t in test_cases.iter() {
            let seed = decode(t.seed).unwrap();

            let master_sk = BlsScalar::from_bytes(
                &t.master_sk
                    .parse::<BigUint>()
                    .unwrap()
                    .to_bytes_le()
                    .try_into()
                    .unwrap(),
            )
            .unwrap();

            let child_index = u32::from_str_radix(t.child_index, 10).unwrap();

            let child_sk = BlsScalar::from_bytes(
                &t.child_sk
                    .parse::<BigUint>()
                    .unwrap()
                    .to_bytes_le()
                    .try_into()
                    .unwrap(),
            )
            .unwrap();

            let derived_master_sk =
                derive_master_sk(&seed).expect("Master SK derivation failed");
            assert_eq!(derived_master_sk, master_sk);

            let derived_sk = derive_child_sk(&master_sk, child_index);
            assert_eq!(derived_sk, child_sk);
        }
    }

    // Tests EIP2333 derivation from path.
    //
    // The seed is produced from a mnemonic phrase and a password. The seed is
    // then used to derive the node secret key corresponding to the given path.
    #[test]
    fn test_path_derivation() {
        let mnemonic = Mnemonic::from_phrase(
          "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about", 
          Language::English
        ).unwrap();

        let seed = Seed::new(&mnemonic, "TREZOR");
        let seed_bytes = seed.as_bytes().try_into().unwrap();

        // Test Cases Set
        // Format: (path, expected_derived_key)
        let test_cases = vec![
          // Test case from Ethereum's reference implementation
          // https://github.com/ethereum/staking-deposit-cli
          ("m/0","20397789859736650942317412262472558107875392172444076792671091975210932703118"),
          // This case has no external reference and only serves as flag for potential breaking changes
          ("m/12381/3600/0/0/0", "1438960529079439298020003172973761593698584351192884838483126814052706935030"),
        ];

        for test in test_cases {
            let path = test.0;
            let child_key = test.1;

            let derived_key = derive_bls_sk(seed_bytes, path).unwrap();

            let expected_key = BlsScalar::from_bytes(
                &(child_key)
                    .parse::<BigUint>()
                    .unwrap()
                    .to_bytes_le()
                    .try_into()
                    .unwrap(),
            )
            .unwrap();

            assert_eq!(derived_key, expected_key);
        }
    }

    // Test path parsing
    #[test]
    fn test_path_parsing() {
        let seed_str = "c55257c360c07c72029aebc1b53c05ed0362ada38ead3e3e9efa3708e53495531f09a6987599d18264c1e1c92f2cf141630c7a3c4ab7c81b2f001698e7463b04";
        let seed_vec = decode(seed_str).unwrap();
        let seed: &[u8; 64] = seed_vec.as_slice().try_into().unwrap();

        // Test cases. Format: (path, expected_result)
        let path_test_cases = vec![
            // Should succeed
            ("m/12381/3600/0/0/0", true),
            // Should fail
            ("x/12381/3600/0/0/0", false),
            ("m/qwert/3600/0/0/0", false),
            ("m/a/3s/1726/0", false),
            ("m", false),
        ];

        for test_case in path_test_cases {
            let path = test_case.0;
            let expected_result = test_case.1;

            // Transform panics in Result
            let result = derive_bls_sk(seed, path);

            assert_eq!(result.is_ok(), expected_result);
        }
    }
}
