// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::env;
use std::num::NonZeroUsize;
use std::sync::{Mutex, MutexGuard, OnceLock};

use lru::LruCache;

macro_rules! define_cache {
    ($get_func:ident, $put_func:ident, $cache_func:ident, $type:ty, $size:literal, $var:literal) => {
        /// Gets an entry out of the cache. Returns `None` if there is no
        /// element in the cache. `Some` signifies that there is a
        /// cache element.
        pub fn $get_func(hash: [u8; blake2b_simd::OUTBYTES]) -> Option<bool> {
            // SAFETY: the closure never panics
            unsafe { $cache_func(|mut cache| cache.get(&hash).copied()) }
        }

        /// Put an entry into the cache.
        pub fn $put_func(hash: [u8; blake2b_simd::OUTBYTES], is_valid: bool) {
            // SAFETY: The closure never panics
            unsafe {
                $cache_func(|mut cache| {
                    cache.put(hash, is_valid);
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
    512,
    "RUSK_ABI_PLONK_CACHE_SIZE"
);
define_cache!(
    get_groth16_verification,
    put_groth16_verification,
    with_groth16_cache,
    bool,
    512,
    "RUSK_ABI_GROTH16_CACHE_SIZE"
);
define_cache!(
    get_bls_verification,
    put_bls_verification,
    with_bls_cache,
    bool,
    512,
    "RUSK_ABI_BLS_CACHE_SIZE"
);
