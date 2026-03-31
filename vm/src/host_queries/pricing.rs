// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Deterministic Boreas host-query pricing.
//!
//! The Boreas gas schedule is derived from pinned single-core measurements on a
//! Hetzner `CPX32` reference provisioner. The pricing workflow for new host
//! queries is:
//! - benchmark the cold path on that reference hardware
//! - use cold `p95` latency as the anchor
//! - apply a `50%` safety multiplier when converting timings into gas
//! - round values into stable protocol buckets instead of encoding benchmark
//!   noise into consensus constants
//! - use flat pricing for fixed-cost crypto queries
//! - use `base + per_byte * bytes` when runtime materially scales with byte
//!   length
//! - keep the resulting schedule compatible with the current `3_000_000_000`
//!   block gas limit and Boreas throughput assumptions

use alloc::boxed::Box;
use alloc::vec::Vec;
use core::any::Any;

use dusk_core::BlsScalar;
use dusk_core::signatures::bls::{
    MultisigSignature, PublicKey as BlsPublicKey,
};
use piecrust::HostQuery;

use super::{
    HardFork, hard_fork, host_hash, host_keccak256, host_poseidon_hash,
    host_secp256k1_recover, host_sha256, host_verify_bls,
    host_verify_bls_multisig, host_verify_groth16_bn254, host_verify_kzg_proof,
    host_verify_plonk, host_verify_schnorr,
};

/// Gas pricing for registered host queries under a specific hardfork.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct HostQueryPricing {
    hash: u64,
    hash_per_byte: u64,
    poseidon_hash_base: u64,
    poseidon_hash_per_scalar: u64,
    verify_plonk: u64,
    verify_groth16_bn254: u64,
    verify_schnorr: u64,
    verify_bls: u64,
    verify_bls_multisig_base: u64,
    verify_bls_multisig_per_key: u64,
    keccak256: u64,
    keccak256_per_byte: u64,
    sha256: u64,
    sha256_per_byte: u64,
    verify_kzg_proof: u64,
    secp256k1_recover: u64,
}

impl HostQueryPricing {
    const fn zero() -> Self {
        Self {
            hash: 0,
            hash_per_byte: 0,
            poseidon_hash_base: 0,
            poseidon_hash_per_scalar: 0,
            verify_plonk: 0,
            verify_groth16_bn254: 0,
            verify_schnorr: 0,
            verify_bls: 0,
            verify_bls_multisig_base: 0,
            verify_bls_multisig_per_key: 0,
            keccak256: 0,
            keccak256_per_byte: 0,
            sha256: 0,
            sha256_per_byte: 0,
            verify_kzg_proof: 0,
            secp256k1_recover: 0,
        }
    }

    /// Rounded Boreas pricing buckets derived from the reference benchmark
    /// workflow documented at the module level.
    const fn boreas_default() -> Self {
        Self {
            hash: 3_000,
            hash_per_byte: 10,
            poseidon_hash_base: 30_000,
            poseidon_hash_per_scalar: 10_000,
            verify_plonk: 10_000_000,
            verify_groth16_bn254: 4_500_000,
            verify_schnorr: 1_000_000,
            verify_bls: 1_750_000,
            verify_bls_multisig_base: 2_500_000,
            verify_bls_multisig_per_key: 200_000,
            keccak256: 3_000,
            keccak256_per_byte: 16,
            sha256: 3_000,
            sha256_per_byte: 8,
            verify_kzg_proof: 2_000_000,
            secp256k1_recover: 200_000,
        }
    }

    fn per_byte_cost(&self, base: u64, per_byte: u64, arg_buf: &[u8]) -> u64 {
        base + per_byte * decoded_vec_len(arg_buf) as u64
    }

    fn per_scalar_cost(
        &self,
        base: u64,
        per_scalar: u64,
        arg_buf: &[u8],
    ) -> u64 {
        base + per_scalar * decoded_scalar_vec_len(arg_buf).max(1) as u64
    }
}

fn decoded_vec_len(arg_buf: &[u8]) -> usize {
    rkyv::from_bytes::<Vec<u8>>(arg_buf)
        .map(|bytes| bytes.len())
        .unwrap_or(arg_buf.len())
}

fn decoded_scalar_vec_len(arg_buf: &[u8]) -> usize {
    rkyv::from_bytes::<Vec<BlsScalar>>(arg_buf)
        .map(|scalars| scalars.len())
        .unwrap_or(1)
}

fn pricing_for_hard_fork(hard_fork: HardFork) -> HostQueryPricing {
    match hard_fork {
        HardFork::PreFork | HardFork::Aegis => HostQueryPricing::zero(),
        HardFork::Boreas => HostQueryPricing::boreas_default(),
    }
}

pub(crate) struct PricedHostQuery {
    price: fn(HostQueryPricing, &[u8]) -> u64,
    execute: fn(&mut [u8], u32) -> u32,
}

impl PricedHostQuery {
    fn new(
        price: fn(HostQueryPricing, &[u8]) -> u64,
        execute: fn(&mut [u8], u32) -> u32,
    ) -> Self {
        Self { price, execute }
    }
}

impl HostQuery for PricedHostQuery {
    fn deserialize_and_price(
        &self,
        arg_buf: &[u8],
        arg: &mut Box<dyn Any>,
    ) -> u64 {
        *arg = Box::new(arg_buf.len() as u32);
        (self.price)(pricing_for_hard_fork(hard_fork()), arg_buf)
    }

    fn execute(&self, arg: &Box<dyn Any>, arg_buf: &mut [u8]) -> u32 {
        let arg_len = *arg.downcast_ref::<u32>().unwrap();
        (self.execute)(arg_buf, arg_len)
    }
}

fn hash_cost(pricing: HostQueryPricing, arg_buf: &[u8]) -> u64 {
    pricing.per_byte_cost(pricing.hash, pricing.hash_per_byte, arg_buf)
}

fn poseidon_hash_cost(pricing: HostQueryPricing, arg_buf: &[u8]) -> u64 {
    pricing.per_scalar_cost(
        pricing.poseidon_hash_base,
        pricing.poseidon_hash_per_scalar,
        arg_buf,
    )
}

fn verify_plonk_cost(pricing: HostQueryPricing, _arg_buf: &[u8]) -> u64 {
    pricing.verify_plonk
}

fn verify_groth16_bn254_cost(
    pricing: HostQueryPricing,
    _arg_buf: &[u8],
) -> u64 {
    pricing.verify_groth16_bn254
}

fn verify_schnorr_cost(pricing: HostQueryPricing, _arg_buf: &[u8]) -> u64 {
    pricing.verify_schnorr
}

fn verify_bls_cost(pricing: HostQueryPricing, _arg_buf: &[u8]) -> u64 {
    pricing.verify_bls
}

fn verify_bls_multisig_cost(pricing: HostQueryPricing, arg_buf: &[u8]) -> u64 {
    let keys =
        rkyv::from_bytes::<(Vec<u8>, Vec<BlsPublicKey>, MultisigSignature)>(
            arg_buf,
        )
        .map(|(_, keys, _)| keys.len() as u64)
        .unwrap_or(1);

    pricing.verify_bls_multisig_base
        + pricing.verify_bls_multisig_per_key * keys.max(1)
}

fn keccak256_cost(pricing: HostQueryPricing, arg_buf: &[u8]) -> u64 {
    pricing.per_byte_cost(
        pricing.keccak256,
        pricing.keccak256_per_byte,
        arg_buf,
    )
}

fn sha256_cost(pricing: HostQueryPricing, arg_buf: &[u8]) -> u64 {
    pricing.per_byte_cost(pricing.sha256, pricing.sha256_per_byte, arg_buf)
}

fn verify_kzg_proof_cost(pricing: HostQueryPricing, _arg_buf: &[u8]) -> u64 {
    pricing.verify_kzg_proof
}

fn secp256k1_recover_cost(pricing: HostQueryPricing, _arg_buf: &[u8]) -> u64 {
    pricing.secp256k1_recover
}

pub(crate) fn hash_host_query() -> PricedHostQuery {
    PricedHostQuery::new(hash_cost, host_hash)
}

pub(crate) fn poseidon_hash_host_query() -> PricedHostQuery {
    PricedHostQuery::new(poseidon_hash_cost, host_poseidon_hash)
}

pub(crate) fn verify_plonk_host_query() -> PricedHostQuery {
    PricedHostQuery::new(verify_plonk_cost, host_verify_plonk)
}

pub(crate) fn verify_groth16_bn254_host_query() -> PricedHostQuery {
    PricedHostQuery::new(verify_groth16_bn254_cost, host_verify_groth16_bn254)
}

pub(crate) fn verify_schnorr_host_query() -> PricedHostQuery {
    PricedHostQuery::new(verify_schnorr_cost, host_verify_schnorr)
}

pub(crate) fn verify_bls_host_query() -> PricedHostQuery {
    PricedHostQuery::new(verify_bls_cost, host_verify_bls)
}

pub(crate) fn verify_bls_multisig_host_query() -> PricedHostQuery {
    PricedHostQuery::new(verify_bls_multisig_cost, host_verify_bls_multisig)
}

pub(crate) fn keccak256_host_query() -> PricedHostQuery {
    PricedHostQuery::new(keccak256_cost, host_keccak256)
}

pub(crate) fn sha256_host_query() -> PricedHostQuery {
    PricedHostQuery::new(sha256_cost, host_sha256)
}

pub(crate) fn verify_kzg_proof_host_query() -> PricedHostQuery {
    PricedHostQuery::new(verify_kzg_proof_cost, host_verify_kzg_proof)
}

pub(crate) fn secp256k1_recover_host_query() -> PricedHostQuery {
    PricedHostQuery::new(secp256k1_recover_cost, host_secp256k1_recover)
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use dusk_core::BlsScalar;
    use dusk_core::signatures::bls::SecretKey as BlsSecretKey;
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    use rkyv::Serialize;
    use rkyv::ser::serializers::AllocSerializer;

    use super::*;

    fn encoded_arg<T>(value: &T) -> Vec<u8>
    where
        T: Serialize<AllocSerializer<1024>>,
    {
        rkyv::to_bytes::<_, 1024>(value).unwrap().to_vec()
    }

    fn expected_boreas_pricing() -> HostQueryPricing {
        HostQueryPricing {
            hash: 3_000,
            hash_per_byte: 10,
            poseidon_hash_base: 30_000,
            poseidon_hash_per_scalar: 10_000,
            verify_plonk: 10_000_000,
            verify_groth16_bn254: 4_500_000,
            verify_schnorr: 1_000_000,
            verify_bls: 1_750_000,
            verify_bls_multisig_base: 2_500_000,
            verify_bls_multisig_per_key: 200_000,
            keccak256: 3_000,
            keccak256_per_byte: 16,
            sha256: 3_000,
            sha256_per_byte: 8,
            verify_kzg_proof: 2_000_000,
            secp256k1_recover: 200_000,
        }
    }

    fn encoded_scalars(count: usize) -> Vec<u8> {
        let scalars: Vec<_> = (0..count)
            .map(|i| BlsScalar::from((i + 1) as u64))
            .collect();
        encoded_arg(&scalars)
    }

    #[test]
    fn pricing_only_activates_at_boreas() {
        assert_eq!(
            pricing_for_hard_fork(HardFork::PreFork),
            HostQueryPricing::zero()
        );
        assert_eq!(
            pricing_for_hard_fork(HardFork::Aegis),
            HostQueryPricing::zero()
        );
        assert_eq!(
            pricing_for_hard_fork(HardFork::Boreas),
            expected_boreas_pricing()
        );
    }

    #[test]
    fn boreas_hash_family_pricing_scales_with_input_bytes() {
        let pricing = pricing_for_hard_fork(HardFork::Boreas);
        let short = encoded_arg(&vec![7u8; 32]);
        let long = encoded_arg(&vec![9u8; 128]);
        let keccak = encoded_arg(&vec![1u8; 96]);
        let sha = encoded_arg(&vec![2u8; 96]);
        let cases = [
            ("hash", hash_cost(pricing, &short), 3_320u64),
            ("hash", hash_cost(pricing, &long), 4_280u64),
            ("keccak256", keccak256_cost(pricing, &keccak), 4_536u64),
            ("sha256", sha256_cost(pricing, &sha), 3_768u64),
        ];

        for (name, actual, expected) in cases {
            assert_eq!(actual, expected, "{name} pricing changed unexpectedly");
        }
        assert!(hash_cost(pricing, &long) > hash_cost(pricing, &short));
    }

    #[test]
    fn boreas_hash_pricing_has_no_word_boundary_cliff() {
        let pricing = pricing_for_hard_fork(HardFork::Boreas);
        let exact_word = encoded_arg(&vec![5u8; 32]);
        let plus_one = encoded_arg(&vec![5u8; 33]);

        assert_eq!(
            hash_cost(pricing, &plus_one) - hash_cost(pricing, &exact_word),
            10
        );
        assert_eq!(
            keccak256_cost(pricing, &plus_one)
                - keccak256_cost(pricing, &exact_word),
            16
        );
        assert_eq!(
            sha256_cost(pricing, &plus_one) - sha256_cost(pricing, &exact_word),
            8
        );
    }

    #[test]
    fn boreas_poseidon_pricing_scales_with_scalar_count() {
        let pricing = pricing_for_hard_fork(HardFork::Boreas);
        let one = encoded_scalars(1);
        let six = encoded_scalars(6);
        let sixteen = encoded_scalars(16);

        assert_eq!(poseidon_hash_cost(pricing, &one), 40_000);
        assert_eq!(poseidon_hash_cost(pricing, &six), 90_000);
        assert_eq!(poseidon_hash_cost(pricing, &sixteen), 190_000);
        assert!(
            poseidon_hash_cost(pricing, &six)
                > poseidon_hash_cost(pricing, &one)
        );
        assert!(
            poseidon_hash_cost(pricing, &sixteen)
                > poseidon_hash_cost(pricing, &six)
        );
    }

    #[test]
    fn boreas_multisig_pricing_scales_with_key_count() {
        let pricing = pricing_for_hard_fork(HardFork::Boreas);
        let msg = b"pricing-multisig";
        let mut rng = StdRng::seed_from_u64(900);
        let sk = BlsSecretKey::random(&mut rng);
        let pk = BlsPublicKey::from(&sk);
        let sig = sk.sign_multisig(&pk, msg);
        let mk_arg =
            |key_count| encoded_arg(&(msg.to_vec(), vec![pk; key_count], sig));

        let two_keys = mk_arg(2);
        let sixteen_keys = mk_arg(16);

        let two_cost = verify_bls_multisig_cost(pricing, &two_keys);
        let sixteen_cost = verify_bls_multisig_cost(pricing, &sixteen_keys);

        assert_eq!(two_cost, 2_900_000);
        assert_eq!(sixteen_cost, 5_700_000);
        assert!(sixteen_cost > two_cost);
    }
}
