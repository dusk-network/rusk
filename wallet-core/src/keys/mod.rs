// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Utilities to derive keys from the seed.

pub mod eip2333;
pub mod eip2334;
pub mod legacy;

// Re-export all phoenix functions, as they are not influenced by EIP-2333
// Temporarily Re-export bls functions as well, until we migrate consuming apps
// to using EIP-2334
pub use legacy::{
    derive_bls_pk, derive_bls_sk, derive_multiple_phoenix_sk,
    derive_phoenix_pk, derive_phoenix_sk, derive_phoenix_vk,
};
