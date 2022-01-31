// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fs;

use rand::rngs::StdRng;
use rand::SeedableRng;
use serde::Serialize;

use dusk_bytes::Serializable;
use dusk_jubjub::BlsScalar;
use dusk_wallet_core::{Store, Wallet};

use crate::lib::clients::{Prover, State};
use crate::lib::crypto::encrypt;
use crate::lib::store::LocalStore;
use crate::lib::{prompt, SEED_SIZE};
use crate::{CliCommand, Error};

mod base64 {
    use serde::{Serialize, Serializer};

    pub fn serialize<S: Serializer>(v: &[u8], s: S) -> Result<S::Ok, S::Error> {
        let base64 = base64::encode(v);
        String::serialize(&base64, s)
    }
}

/// Bls key pair helper structure
#[derive(Serialize)]
struct BlsKeyPair {
    #[serde(with = "base64")]
    secret_key_bls: [u8; 32],
    #[serde(with = "base64")]
    public_key_bls: [u8; 96],
}

/// Interface to wallet_core lib
pub(crate) struct CliWallet {
    store: LocalStore,
    wallet: Wallet<LocalStore, State, Prover>,
}

impl CliWallet {
    /// Creates a new CliWallet instance
    pub fn new(store: LocalStore, state: State, prover: Prover) -> Self {
        CliWallet {
            store: store.clone(),
            wallet: Wallet::new(store, state, prover),
        }
    }

    /// Runs the CliWallet in interactive mode
    pub fn interactive(&self) -> Result<(), Error> {
        loop {
            match prompt::command() {
                Some(cmd) => self.run(cmd)?,
                None => return Ok(()),
            }
        }
    }

    /// Runs a command through wallet core lib
    pub fn run(&self, cmd: CliCommand) -> Result<(), Error> {
        // perform whatever action user requested
        use CliCommand::*;
        match cmd {
            // Check your current balance
            Balance { key } => {
                let balance = self.wallet.get_balance(key)?;
                println!(
                    "Balance for key {} is: {} Dusk ✅",
                    key,
                    balance / 1_000_000
                );
            }

            // Retrieve public spend key
            Address { key } => {
                let pk = self.wallet.public_spend_key(key)?;
                let addr = pk.to_bytes();
                let addr = bs58::encode(addr).into_string();
                println!("Public address for key {} is: {:?} ✅", key, addr);
            }

            // Send Dusk through the network
            Transfer {
                key,
                rcvr,
                amt,
                gas_limit,
                gas_price,
            } => {
                let mut addr_bytes = [0u8; SEED_SIZE];
                addr_bytes.copy_from_slice(&bs58::decode(rcvr).into_vec()?);
                let dest_addr =
                    dusk_pki::PublicSpendKey::from_bytes(&addr_bytes)?;
                let my_addr = self.wallet.public_spend_key(key)?;
                let mut rng = StdRng::from_entropy();
                self.wallet.transfer(
                    &mut rng,
                    key,
                    &my_addr,
                    &dest_addr,
                    amt,
                    gas_limit,
                    gas_price.unwrap_or(0),
                    BlsScalar::zero(),
                )?;
                println!("Transfer sent! ✅");
            }

            // Start staking Dusk
            Stake {
                key,
                stake_key,
                amt,
                gas_limit,
                gas_price,
            } => {
                let my_addr = self.wallet.public_spend_key(key)?;
                let mut rng = StdRng::from_entropy();
                self.wallet.stake(
                    &mut rng,
                    key,
                    stake_key,
                    &my_addr,
                    amt,
                    gas_limit,
                    gas_price.unwrap_or(0),
                )?;
                println!("Stake success! ✅");
            }

            // Extend stake for a particular key
            ExtendStake {
                key,
                stake_key,
                gas_limit,
                gas_price,
            } => {
                let my_addr = self.wallet.public_spend_key(key)?;
                let mut rng = StdRng::from_entropy();
                self.wallet.extend_stake(
                    &mut rng,
                    key,
                    stake_key,
                    &my_addr,
                    gas_limit,
                    gas_price.unwrap_or(0),
                )?;
                println!("Stake extension success! ✅");
            }

            // Withdraw a key's stake
            WithdrawStake {
                key,
                stake_key,
                gas_limit,
                gas_price,
            } => {
                let my_addr = self.wallet.public_spend_key(key)?;
                let mut rng = StdRng::from_entropy();
                self.wallet.withdraw_stake(
                    &mut rng,
                    key,
                    stake_key,
                    &my_addr,
                    gas_limit,
                    gas_price.unwrap_or(0),
                )?;
                println!("Stake withdrawal success! ✅");
            }

            Export { key, plaintext } => {
                // retrieve keys
                let sk = self.store.retrieve_sk(key)?;
                let pk = self.wallet.public_key(key)?;

                // create node-compatible json structure
                let bls = BlsKeyPair {
                    secret_key_bls: sk.to_bytes(),
                    public_key_bls: pk.to_bytes(),
                };
                let json = serde_json::to_string(&bls)?;

                // encrypt data
                let mut bytes = json.as_bytes().to_vec();
                if !plaintext {
                    let pwd = prompt::request_auth("Encryption password");
                    bytes = encrypt(&bytes, pwd)?;
                }

                // write to disk
                let filename = match self.store.name() {
                    Some(name) => format!("{}-{}", name, key),
                    None => key.to_string(),
                };

                let mut path = dirs::home_dir().expect("user home dir");
                path.push(&filename);
                path.set_extension("key");

                fs::write(&path, bytes)?;

                println!(
                    "Key pair exported to {} ✅",
                    path.as_os_str().to_str().unwrap()
                )
            }

            // Do nothing
            _ => {}
        }

        Ok(())
    }
}
