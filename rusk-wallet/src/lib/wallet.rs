// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::{fs, thread, time::Duration};

use rand::rngs::StdRng;
use rand::SeedableRng;
use serde::Serialize;

use dusk_bytes::Serializable;
use dusk_jubjub::BlsScalar;
use dusk_wallet_core::{Store, Wallet};

use crate::lib::clients::{Prover, State};
use crate::lib::crypto::encrypt;
use crate::lib::store::LocalStore;
use crate::lib::{prompt, DEFAULT_GAS_PRICE, SEED_SIZE};
use crate::lib::{to_dusk, to_udusk};
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
    wallet: Option<Wallet<LocalStore, State, Prover>>,
}

impl CliWallet {
    /// Creates a new CliWallet instance
    pub fn new(store: LocalStore, state: State, prover: Prover) -> Self {
        CliWallet {
            store: store.clone(),
            wallet: Some(Wallet::new(store, state, prover)),
        }
    }

    /// Creates a new offline CliWallet instance
    pub fn offline(store: LocalStore) -> Self {
        CliWallet {
            store,
            wallet: None,
        }
    }

    /// Runs the CliWallet in interactive mode
    pub fn interactive(&self) -> Result<(), Error> {
        let offline = self.wallet.is_none();
        loop {
            use prompt::PromptCommand;
            match prompt::choose_command(offline) {
                Some(pcmd) => {
                    // load key balance first to provide interactive feedback
                    let balance = if let Some(wallet) = &self.wallet {
                        match pcmd {
                            PromptCommand::Export => 0,
                            PromptCommand::Address(key) => {
                                wallet.get_balance(key)?
                            }
                            PromptCommand::Balance(key) => {
                                wallet.get_balance(key)?
                            }
                            PromptCommand::Transfer(key) => {
                                wallet.get_balance(key)?
                            }
                            PromptCommand::Stake(key) => {
                                wallet.get_balance(key)?
                            }
                            PromptCommand::Withdraw(key) => {
                                wallet.get_balance(key)?
                            }
                        }
                    } else {
                        0
                    };

                    // prepare command
                    let cmd = prompt::prepare_command(pcmd, to_dusk(balance));
                    // run command
                    self.run(cmd)?;
                    // wait for a second
                    thread::sleep(Duration::from_millis(1000));
                    println!("—")
                }
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
                if let Some(wallet) = &self.wallet {
                    let balance = wallet.get_balance(key)?;
                    println!(
                        "> Balance for key {} is: {} Dusk",
                        key,
                        to_dusk(balance)
                    );
                    Ok(())
                } else {
                    Err(Error::Offline)
                }
            }

            // Retrieve public spend key
            Address { key } => {
                let pk = if let Some(wallet) = &self.wallet {
                    wallet.public_spend_key(key)?
                } else {
                    let ssk = self.store.retrieve_ssk(key)?;
                    ssk.public_spend_key()
                };
                let addr = pk.to_bytes();
                let addr = bs58::encode(addr).into_string();
                println!("> Public address for key {} is: {}", key, addr);
                Ok(())
            }

            // Send Dusk through the network
            Transfer {
                key,
                rcvr,
                amt,
                gas_limit,
                gas_price,
            } => {
                if let Some(wallet) = &self.wallet {
                    let mut addr_bytes = [0u8; SEED_SIZE];
                    addr_bytes.copy_from_slice(&bs58::decode(rcvr).into_vec()?);
                    let dest_addr =
                        dusk_pki::PublicSpendKey::from_bytes(&addr_bytes)?;
                    let my_addr = wallet.public_spend_key(key)?;

                    let mut rng = StdRng::from_entropy();
                    let ref_id = BlsScalar::random(&mut rng);

                    wallet.transfer(
                        &mut rng,
                        key,
                        &my_addr,
                        &dest_addr,
                        amt,
                        gas_limit,
                        gas_price
                            .unwrap_or_else(|| to_udusk(DEFAULT_GAS_PRICE)),
                        ref_id,
                    )?;
                    println!("> Transfer sent!");
                    Ok(())
                } else {
                    Err(Error::Offline)
                }
            }

            // Start staking Dusk
            Stake {
                key,
                stake_key,
                amt,
                gas_limit,
                gas_price,
            } => {
                if let Some(wallet) = &self.wallet {
                    let my_addr = wallet.public_spend_key(key)?;
                    let mut rng = StdRng::from_entropy();
                    wallet.stake(
                        &mut rng,
                        key,
                        stake_key,
                        &my_addr,
                        amt,
                        gas_limit,
                        gas_price
                            .unwrap_or_else(|| to_udusk(DEFAULT_GAS_PRICE)),
                    )?;
                    println!("> Stake success!");
                    Ok(())
                } else {
                    Err(Error::Offline)
                }
            }

            // Withdraw a key's stake
            WithdrawStake {
                key,
                stake_key,
                gas_limit,
                gas_price,
            } => {
                if let Some(wallet) = &self.wallet {
                    let my_addr = wallet.public_spend_key(key)?;
                    let mut rng = StdRng::from_entropy();
                    wallet.withdraw_stake(
                        &mut rng,
                        key,
                        stake_key,
                        &my_addr,
                        gas_limit,
                        gas_price
                            .unwrap_or_else(|| to_udusk(DEFAULT_GAS_PRICE)),
                    )?;
                    println!("> Stake withdrawal success!");
                    Ok(())
                } else {
                    Err(Error::Offline)
                }
            }

            Export { key, plaintext } => {
                // retrieve keys
                let sk = self.store.retrieve_sk(key)?;
                let pk = if let Some(wallet) = &self.wallet {
                    wallet.public_key(key)?
                } else {
                    From::from(&sk)
                };

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

                // add wallet name to file
                let filename = match self.store.name() {
                    Some(name) => format!("{}-{}", name, key),
                    None => key.to_string(),
                };

                // write to disk
                let mut path = dirs::home_dir().expect("user home dir");
                path.push(&filename);
                path.set_extension("key");

                fs::write(&path, bytes)?;

                println!(
                    "> Key pair exported to {}",
                    path.as_os_str().to_str().unwrap()
                );
                Ok(())
            }

            // Do nothing
            _ => Ok(()),
        }
    }
}
