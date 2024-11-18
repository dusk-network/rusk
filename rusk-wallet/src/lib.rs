// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! # Dusk Wallet Lib
//!
//! The `dusk_wallet` library aims to provide an easy and convenient way of
//! interfacing with the Dusk Network.
//!
//! Clients can use `Wallet` to create their Dusk wallet, send transactions
//! through the network of their choice, stake and withdraw rewards, etc.

#![deny(missing_docs)]

mod cache;
mod clients;
mod crypto;
mod error;
mod gql;
mod rues;
mod store;
mod wallet;

pub mod currency;
pub mod dat;
pub mod gas;

pub use rues::RuesHttpClient;

pub use error::Error;
pub use wallet::{
    Address, DecodedNote, Profile, SecureWalletFile, Wallet, WalletPath,
};

pub use gql::{BlockTransaction, GraphQL};

use execution_core::{
    dusk, from_dusk,
    stake::StakeData,
    transfer::phoenix::{
        ArchivedNoteLeaf, Note, NoteOpening, PublicKey as PhoenixPublicKey,
        SecretKey as PhoenixSecretKey, ViewKey as PhoenixViewKey,
    },
    BlsScalar,
};

use currency::Dusk;

/// The maximum allowed size for function names, set to 64 bytes
pub const MAX_FUNCTION_NAME_SIZE: usize = 64;
/// The largest amount of Dusk that is possible to convert
pub const MAX_CONVERTIBLE: Dusk = Dusk::MAX;
/// The smallest amount of Dusk that is possible to convert
pub const MIN_CONVERTIBLE: Dusk = Dusk::new(1);
/// The length of an epoch in blocks
pub const EPOCH: u64 = 2160;
/// Max addresses the wallet can store
pub const MAX_PROFILES: usize = get_max_profiles();

const DEFAULT_MAX_PROFILES: usize = 2;

// PANIC: the function is const and will panic during compilation if the value
// is invalid
const fn get_max_profiles() -> usize {
    match option_env!("WALLET_MAX_PROFILES") {
        Some(v) => match konst::primitive::parse_usize(v) {
            Ok(e) if e > 255 => {
                panic!("WALLET_MAX_PROFILES must be lower or equal to 255")
            }
            Ok(e) if e > 0 => e,
            _ => panic!("Invalid WALLET_MAX_PROFILES"),
        },
        None => DEFAULT_MAX_PROFILES,
    }
}
