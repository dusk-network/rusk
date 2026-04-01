// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use core::cell::Cell;
use std::env;
use std::num::NonZeroUsize;
use std::sync::{Mutex, MutexGuard, OnceLock};

use dusk_core::plonk::PlonkVersion;
use lru::LruCache;

use super::HardFork;

// These caches are process-global and survive across block executions.
// Cache keys for consensus-sensitive host queries must therefore include every
// execution-policy input that can change semantics across heights or releases.

const fn plonk_cache_revision(version: PlonkVersion) -> u32 {
    match version {
        PlonkVersion::V1 => 1,
        PlonkVersion::V2 => 2,
        PlonkVersion::V3 => 3,
        _ => u32::MAX,
    }
}

const fn bls_cache_revision(hard_fork: HardFork) -> u32 {
    match hard_fork {
        HardFork::PreFork => 0,
        HardFork::Aegis | HardFork::Boreas => 1,
    }
}

/// Active execution policy used by VM host queries.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct HostQueryPolicy {
    /// PLONK verifier version selected for this execution context.
    pub plonk_version: PlonkVersion,
    /// Active hardfork used for BLS verification semantics.
    pub hard_fork: HardFork,
}

impl HostQueryPolicy {
    /// Creates a policy from the active PLONK and hardfork versions.
    pub const fn from_versions(
        plonk_version: PlonkVersion,
        hard_fork: HardFork,
    ) -> Self {
        Self {
            plonk_version,
            hard_fork,
        }
    }
}

thread_local! {
    // Default to V2 for safety: if the node forgets to set a version for a
    // consensus-critical call path, we'd rather reject than accept.
    static HOST_QUERY_POLICY: Cell<HostQueryPolicy> = const {
        Cell::new(HostQueryPolicy::from_versions(
            PlonkVersion::V2,
            HardFork::PreFork,
        ))
    };
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(super) enum CacheDomain {
    Hash,
    PoseidonHash,
    Plonk,
    Groth16Bn254,
    Schnorr,
    Bls,
    BlsMultisig,
    Keccak256,
    Sha256,
    Kzg,
    Secp256k1Recover,
}

impl CacheDomain {
    const fn tag(self) -> u8 {
        match self {
            CacheDomain::Hash => 0,
            CacheDomain::PoseidonHash => 1,
            CacheDomain::Plonk => 2,
            CacheDomain::Groth16Bn254 => 3,
            CacheDomain::Schnorr => 4,
            CacheDomain::Bls => 5,
            CacheDomain::BlsMultisig => 6,
            CacheDomain::Keccak256 => 7,
            CacheDomain::Sha256 => 8,
            CacheDomain::Kzg => 9,
            CacheDomain::Secp256k1Recover => 10,
        }
    }
}

// Cache revisions must be derived from execution policy inputs that change at
// explicit activation boundaries. That keeps long-lived nodes aligned with
// fresh nodes across fork and feature transitions.
const fn cache_revision(policy: HostQueryPolicy, domain: CacheDomain) -> u32 {
    match domain {
        CacheDomain::Plonk => plonk_cache_revision(policy.plonk_version),
        CacheDomain::Bls | CacheDomain::BlsMultisig => {
            bls_cache_revision(policy.hard_fork)
        }
        CacheDomain::Hash
        | CacheDomain::PoseidonHash
        | CacheDomain::Groth16Bn254
        | CacheDomain::Schnorr
        | CacheDomain::Keccak256
        | CacheDomain::Sha256
        | CacheDomain::Kzg
        | CacheDomain::Secp256k1Recover => 0,
    }
}

/// Guard that restores the previous host-query policy when dropped.
#[derive(Debug)]
pub struct HostQueryPolicyGuard {
    prev: HostQueryPolicy,
}

impl Drop for HostQueryPolicyGuard {
    fn drop(&mut self) {
        HOST_QUERY_POLICY.with(|m| m.set(self.prev));
    }
}

/// Returns the current thread's host-query policy.
pub fn host_query_policy() -> HostQueryPolicy {
    HOST_QUERY_POLICY.with(|m| m.get())
}

/// Sets the current thread's host-query policy.
///
/// The previous policy is restored when the returned guard is dropped.
pub fn set_host_query_policy(policy: HostQueryPolicy) -> HostQueryPolicyGuard {
    let prev = HOST_QUERY_POLICY.with(|m| {
        let prev = m.get();
        m.set(policy);
        prev
    });
    HostQueryPolicyGuard { prev }
}

/// Returns the current thread's PLONK version (defaults to `V2`).
pub fn plonk_version() -> PlonkVersion {
    host_query_policy().plonk_version
}

/// Returns the active hardfork for this thread.
pub fn hard_fork() -> HardFork {
    host_query_policy().hard_fork
}

pub(super) fn cache_key_with_revision(
    domain: CacheDomain,
    revision: u32,
    arg_buf: &[u8],
) -> [u8; blake2b_simd::OUTBYTES] {
    // Domain-separate cache entries by query domain and semantics revision.
    let mut state = blake2b_simd::Params::new()
        .hash_length(blake2b_simd::OUTBYTES)
        .to_state();
    state.update(&[domain.tag()]);
    state.update(&revision.to_le_bytes());
    state.update(arg_buf);
    *state.finalize().as_array()
}

pub(super) fn cache_key(
    domain: CacheDomain,
    arg_buf: &[u8],
) -> [u8; blake2b_simd::OUTBYTES] {
    let revision = cache_revision(host_query_policy(), domain);
    cache_key_with_revision(domain, revision, arg_buf)
}

type ScalarCacheValue = dusk_core::BlsScalar;
type RecoverCacheValue = Option<[u8; 65]>;

macro_rules! define_cache {
    ($get_func:ident, $put_func:ident, $cache_func:ident, $type:ty, $size:literal, $var:literal) => {
        /// Gets an entry out of the cache. Returns `None` if there is no
        /// element in the cache. `Some` signifies that there is a
        /// cache element.
        pub fn $get_func(hash: [u8; blake2b_simd::OUTBYTES]) -> Option<$type> {
            // SAFETY: the closure never panics
            unsafe { $cache_func(|mut cache| cache.get(&hash).cloned()) }
        }

        /// Put an entry into the cache.
        pub fn $put_func(hash: [u8; blake2b_simd::OUTBYTES], value: $type) {
            // SAFETY: The closure never panics
            unsafe {
                $cache_func(|mut cache| {
                    cache.put(hash, value);
                });
            }
        }

        /// A simple LRU cache.
        ///
        /// # Safety
        /// `f` should *never* panic, otherwise we poison the Mutex.
        unsafe fn $cache_func<T, F>(f: F) -> T
        where
            F: FnOnce(
                MutexGuard<LruCache<[u8; blake2b_simd::OUTBYTES], $type>>,
            ) -> T,
        {
            const DEFAULT_SIZE: usize = $size;

            static CACHE: OnceLock<
                Mutex<LruCache<[u8; blake2b_simd::OUTBYTES], $type>>,
            > = OnceLock::new();

            CACHE
                .get_or_init(|| {
                    let mut cache_size = None;

                    if let Ok(s) = env::var($var) {
                        cache_size = s.parse().ok();
                    }

                    let mut cache_size = cache_size.unwrap_or(DEFAULT_SIZE);
                    if cache_size == 0 {
                        cache_size = DEFAULT_SIZE;
                    }

                    Mutex::new(LruCache::new(
                        NonZeroUsize::new(cache_size).unwrap(),
                    ))
                })
                .lock()
                .map(f)
                .unwrap()
        }
    };
}

define_cache!(
    get_plonk_verification,
    put_plonk_verification,
    with_plonk_cache,
    bool,
    2048,
    "DUSK_VM_PLONK_CACHE_SIZE"
);
define_cache!(
    get_groth16_verification,
    put_groth16_verification,
    with_groth16_cache,
    bool,
    2048,
    "DUSK_VM_GROTH16_CACHE_SIZE"
);
define_cache!(
    get_bls_verification,
    put_bls_verification,
    with_bls_cache,
    bool,
    2048,
    "DUSK_VM_BLS_CACHE_SIZE"
);
define_cache!(
    get_hash,
    put_hash,
    with_hash_cache,
    ScalarCacheValue,
    2048,
    "DUSK_VM_HASH_CACHE_SIZE"
);
define_cache!(
    get_poseidon_hash,
    put_poseidon_hash,
    with_poseidon_hash_cache,
    ScalarCacheValue,
    2048,
    "DUSK_VM_POSEIDON_HASH_CACHE_SIZE"
);
define_cache!(
    get_schnorr_verification,
    put_schnorr_verification,
    with_schnorr_cache,
    bool,
    2048,
    "DUSK_VM_SCHNORR_CACHE_SIZE"
);
define_cache!(
    get_bls_multisig_verification,
    put_bls_multisig_verification,
    with_bls_multisig_cache,
    bool,
    2048,
    "DUSK_VM_BLS_MULTISIG_CACHE_SIZE"
);
define_cache!(
    get_keccak256,
    put_keccak256,
    with_keccak256_cache,
    [u8; 32],
    2048,
    "DUSK_VM_KECCAK256_CACHE_SIZE"
);
define_cache!(
    get_sha256,
    put_sha256,
    with_sha256_cache,
    [u8; 32],
    2048,
    "DUSK_VM_SHA256_CACHE_SIZE"
);
define_cache!(
    get_kzg_verification,
    put_kzg_verification,
    with_kzg_cache,
    bool,
    2048,
    "DUSK_VM_KZG_CACHE_SIZE"
);
define_cache!(
    get_secp256k1_recover,
    put_secp256k1_recover,
    with_secp256k1_recover_cache,
    RecoverCacheValue,
    2048,
    "DUSK_VM_SECP256K1_RECOVER_CACHE_SIZE"
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_query_policy_revisions_follow_versions() {
        let prefork =
            HostQueryPolicy::from_versions(PlonkVersion::V1, HardFork::PreFork);
        let aegis =
            HostQueryPolicy::from_versions(PlonkVersion::V3, HardFork::Aegis);
        let boreas =
            HostQueryPolicy::from_versions(PlonkVersion::V3, HardFork::Boreas);

        assert_ne!(
            cache_revision(prefork, CacheDomain::Plonk),
            cache_revision(aegis, CacheDomain::Plonk)
        );
        assert_ne!(
            cache_revision(prefork, CacheDomain::Bls),
            cache_revision(aegis, CacheDomain::Bls)
        );
        assert_eq!(
            cache_revision(aegis, CacheDomain::Bls),
            cache_revision(aegis, CacheDomain::BlsMultisig)
        );
        assert_eq!(
            cache_revision(aegis, CacheDomain::Bls),
            cache_revision(boreas, CacheDomain::Bls)
        );
        assert_eq!(cache_revision(prefork, CacheDomain::Schnorr), 0);
    }

    #[test]
    fn cache_key_is_domain_and_revision_separated() {
        let arg = [1u8, 2, 3, 4];

        let hash_key = cache_key_with_revision(CacheDomain::Hash, 0, &arg);
        let schnorr_key =
            cache_key_with_revision(CacheDomain::Schnorr, 0, &arg);
        let bumped_hash_key =
            cache_key_with_revision(CacheDomain::Hash, 1, &arg);

        assert_ne!(hash_key, schnorr_key);
        assert_ne!(hash_key, bumped_hash_key);
    }

    #[test]
    fn set_host_query_policy_restores_previous_policy() {
        let prev = host_query_policy();
        let next =
            HostQueryPolicy::from_versions(PlonkVersion::V3, HardFork::Boreas);

        {
            let _guard = set_host_query_policy(next);
            assert_eq!(host_query_policy(), next);
        }

        assert_eq!(host_query_policy(), prev);
    }

    #[test]
    fn host_query_policy_updates_cache_revisions() {
        let prev = host_query_policy();

        {
            let _guard = set_host_query_policy(HostQueryPolicy::from_versions(
                PlonkVersion::V1,
                prev.hard_fork,
            ));
            assert_eq!(plonk_version(), PlonkVersion::V1);
            assert_eq!(
                cache_revision(host_query_policy(), CacheDomain::Plonk),
                plonk_cache_revision(PlonkVersion::V1)
            );
        }

        assert_eq!(host_query_policy(), prev);

        {
            let _guard = set_host_query_policy(HostQueryPolicy::from_versions(
                prev.plonk_version,
                HardFork::Boreas,
            ));
            assert_eq!(hard_fork(), HardFork::Boreas);
            assert_eq!(
                cache_revision(host_query_policy(), CacheDomain::Bls),
                bls_cache_revision(HardFork::Boreas)
            );
            assert_eq!(
                cache_revision(host_query_policy(), CacheDomain::BlsMultisig),
                bls_cache_revision(HardFork::Boreas)
            );
        }

        assert_eq!(host_query_policy(), prev);
    }
}
