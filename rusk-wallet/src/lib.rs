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

/// module to help with currency conversions
pub mod currency;

mod block;
mod cache;
mod clients;
mod crypto;
mod error;
mod rusk;
mod store;
mod wallet;

/// Methods for parsing/checking the DAT wallet file
pub mod dat;

pub use rusk::{RuskHttpClient, RuskRequest};

pub use error::Error;
pub use wallet::gas;
pub use wallet::{Address, DecodedNote, SecureWalletFile, Wallet, WalletPath};

use execution_core::{
    dusk, from_dusk,
    signatures::bls::PublicKey as AccountPublicKey,
    stake::StakeData,
    transfer::phoenix::{
        ArchivedNoteLeaf, Note, NoteLeaf, NoteOpening,
        PublicKey as PhoenixPublicKey, SecretKey as PhoenixSecretKey,
        ViewKey as PhoenixViewKey,
    },
    BlsScalar,
};

use currency::Dusk;

/// The largest amount of Dusk that is possible to convert
pub const MAX_CONVERTIBLE: Dusk = Dusk::MAX;
/// The smallest amount of Dusk that is possible to convert
pub const MIN_CONVERTIBLE: Dusk = Dusk::new(1);
/// The length of an epoch in blocks
pub const EPOCH: u64 = 2160;
/// Max addresses the wallet can store
pub const MAX_ADDRESSES: usize = get_max_addresses();

const DEFAULT_MAX_ADDRESSES: usize = 2;

const fn get_max_addresses() -> usize {
    match option_env!("WALLET_MAX_ADDR") {
        Some(v) => match konst::primitive::parse_usize(v) {
            Ok(e) if e > 255 => {
                panic!("WALLET_MAX_ADDR must be lower or equal to 255")
            }
            Ok(e) if e > 0 => e,
            _ => panic!("Invalid WALLET_MAX_ADDR"),
        },
        None => DEFAULT_MAX_ADDRESSES,
    }
}
