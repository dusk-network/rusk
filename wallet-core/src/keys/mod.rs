// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Utilities to derive keys from the seed.

pub mod eip2333;
pub mod eip2334;
pub mod legacy;
pub mod phoenix_hd;

// Re-export legacy APIs for backward compatibility while consumers migrate.
pub use legacy::{
    derive_bls_pk, derive_bls_sk, derive_multiple_phoenix_sk,
    derive_phoenix_pk, derive_phoenix_sk, derive_phoenix_vk,
};

// Re-export Phoenix HD v1 account APIs.
pub use phoenix_hd::{
    phoenix_account, phoenix_dual_scan_accounts, phoenix_legacy_account,
    phoenix_master_from_seed, phoenix_migration_account, PhoenixAccount,
    PhoenixDerivationPath, PhoenixHdError, PhoenixLegacyAccount,
    PhoenixMasterKey, PhoenixMigrationAccount, DUSK_COIN_TYPE, PHOENIX_PURPOSE,
};
