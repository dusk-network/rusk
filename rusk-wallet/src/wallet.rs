// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod address;
mod file;
pub mod gas;

pub use address::Address;
pub use file::{SecureWalletFile, WalletPath};

use bip39::{Language, Mnemonic, Seed};
use dusk_bytes::Serializable;
use rand::rngs::StdRng;
use rand::SeedableRng;

use rues::RuesHttpClient;
use serde::Serialize;
use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};

use wallet_core::{
    phoenix_balance,
    prelude::keys::{
        derive_bls_pk, derive_bls_sk, derive_phoenix_pk, derive_phoenix_sk,
        derive_phoenix_vk,
    },
    transaction::{
        moonlight, moonlight_stake, moonlight_to_phoenix, moonlight_unstake,
        phoenix, phoenix_stake, phoenix_stake_reward, phoenix_to_moonlight,
        phoenix_unstake,
    },
    BalanceInfo,
};

use execution_core::{
    signatures::bls::{PublicKey as BlsPublicKey, SecretKey as BlsSecretKey},
    transfer::{data::ContractCall, data::TransactionData, Transaction},
};

use zeroize::Zeroize;

use super::*;

use crate::{
    cache::NoteData,
    clients::{Prover, State},
    crypto::encrypt,
    currency::Dusk,
    dat::{
        self, version_bytes, DatFileVersion, FILE_TYPE, LATEST_VERSION, MAGIC,
        RESERVED,
    },
    store::LocalStore,
    Error, RuskHttpClient,
};

use gas::Gas;

/// The interface to the Dusk Network
///
/// The Wallet exposes all methods available to interact with the Dusk Network.
///
/// A new [`Wallet`] can be created from a bip39-compatible mnemonic phrase or
/// an existing wallet file.
///
/// The user can generate as many [`Address`] as needed without an active
/// connection to the network by calling [`Wallet::new_address`] repeatedly.
///
/// A wallet must connect to the network using a [`RuskEndpoint`] in order to be
/// able to perform common operations such as checking balance, transfernig
/// funds, or staking Dusk.
pub struct Wallet<F: SecureWalletFile + Debug> {
    addresses: Vec<Address>,
    state: Option<State>,
    store: LocalStore,
    file: Option<F>,
    file_version: Option<DatFileVersion>,
}

impl<F: SecureWalletFile + Debug> Wallet<F> {
    /// Returns the file used for the wallet
    pub fn file(&self) -> &Option<F> {
        &self.file
    }

    /// Returns phoenix key pair for a given address
    ///
    /// # Errors
    ///
    /// - If the Address provided is not a Phoenix address
    /// - If the address is not owned
    pub fn phoenix_keys(
        &self,
        addr: &Address,
    ) -> Result<(PhoenixPublicKey, PhoenixSecretKey), Error> {
        // make sure we own the address
        if !addr.is_owned() {
            return Err(Error::Unauthorized);
        }

        let index = addr.index()?;

        // retrieve keys
        let sk = self.phoenix_secret_key(index);
        let pk = addr.pk()?;

        Ok((*pk, sk))
    }
}

impl<F: SecureWalletFile + Debug> Wallet<F> {
    /// Creates a new wallet instance deriving its seed from a valid BIP39
    /// mnemonic
    pub fn new<P>(phrase: P) -> Result<Self, Error>
    where
        P: Into<String>,
    {
        // generate mnemonic
        let phrase: String = phrase.into();
        let try_mnem = Mnemonic::from_phrase(&phrase, Language::English);

        if let Ok(mnemonic) = try_mnem {
            // derive the mnemonic seed
            let seed = Seed::new(&mnemonic, "");
            // Takes the mnemonic seed as bytes
            let bytes = seed.as_bytes().try_into().unwrap();

            // Generate the default address
            let address = Address::Phoenix {
                index: Some(0),
                addr: derive_phoenix_pk(&bytes, 0),
            };

            // return new wallet instance
            Ok(Wallet {
                addresses: vec![address],
                state: None,
                store: LocalStore::from(bytes),
                file: None,
                file_version: None,
            })
        } else {
            Err(Error::InvalidMnemonicPhrase)
        }
    }

    /// Loads wallet given a session
    pub fn from_file(file: F) -> Result<Self, Error> {
        let path = file.path();
        let pwd = file.pwd();

        // make sure file exists
        let pb = path.inner().clone();
        if !pb.is_file() {
            return Err(Error::WalletFileMissing);
        }

        // attempt to load and decode wallet
        let bytes = fs::read(&pb)?;

        let file_version = dat::check_version(bytes.get(0..12))?;

        let (seed, address_count) =
            dat::get_seed_and_address(file_version, bytes, pwd)?;

        // return early if its legacy
        if let DatFileVersion::Legacy = file_version {
            let address = Address::Phoenix {
                index: Some(0),
                addr: derive_phoenix_pk(&seed, 0),
            };

            // return the store
            return Ok(Self {
                addresses: vec![address],
                store: LocalStore::from(seed),
                state: None,
                file: Some(file),
                file_version: Some(DatFileVersion::Legacy),
            });
        }

        let addresses: Vec<_> = (0..address_count)
            .map(|i| Address::Phoenix {
                index: Some(i),
                addr: derive_phoenix_pk(&seed, i),
            })
            .collect();

        // create and return
        Ok(Self {
            addresses,
            store: LocalStore::from(seed),
            state: None,
            file: Some(file),
            file_version: Some(file_version),
        })
    }

    /// Saves wallet to file from which it was loaded
    pub fn save(&mut self) -> Result<(), Error> {
        match &self.file {
            Some(f) => {
                let mut header = Vec::with_capacity(12);
                header.extend_from_slice(&MAGIC.to_be_bytes());
                // File type = Rusk Wallet (0x02)
                header.extend_from_slice(&FILE_TYPE.to_be_bytes());
                // Reserved (0x0)
                header.extend_from_slice(&RESERVED.to_be_bytes());
                // Version
                header.extend_from_slice(&version_bytes(LATEST_VERSION));

                // create file payload
                let seed = self.store.get_seed();
                let mut payload = seed.to_vec();

                payload.push(self.addresses.len() as u8);

                // encrypt the payload
                payload = encrypt(&payload, f.pwd())?;

                let mut content =
                    Vec::with_capacity(header.len() + payload.len());

                content.extend_from_slice(&header);
                content.extend_from_slice(&payload);

                // write the content to file
                fs::write(&f.path().wallet, content)?;
                Ok(())
            }
            None => Err(Error::WalletFileMissing),
        }
    }

    /// Saves wallet to the provided file, changing the previous file path for
    /// the wallet if any. Note that any subsequent calls to [`save`] will
    /// use this new file.
    pub fn save_to(&mut self, file: F) -> Result<(), Error> {
        // set our new file and save
        self.file = Some(file);
        self.save()
    }

    /// Access the inner state of the wallet
    pub fn state(&self) -> Result<&State, Error> {
        if let Some(state) = self.state.as_ref() {
            Ok(state)
        } else {
            Err(Error::Offline)
        }
    }

    /// Connect the wallet to the network providing a callback for status
    /// updates
    pub async fn connect_with_status<S>(
        &mut self,
        rusk_addr: S,
        prov_addr: S,
        status: fn(&str),
    ) -> Result<(), Error>
    where
        S: Into<String>,
    {
        // attempt connection
        let http_state = RuesHttpClient::new(rusk_addr.into());
        let http_prover = RuskHttpClient::new(prov_addr.into());

        let state_status = http_state.check_connection().await;
        let prover_status = http_prover.check_connection().await;

        match (&state_status, prover_status) {
            (Err(e),_)=> println!("Connection to Rusk Failed, some operations won't be available: {e}"),
            (_,Err(e))=> println!("Connection to Prover Failed, some operations won't be available: {e}"),
            _=> {},
        }

        let cache_dir = {
            if let Some(file) = &self.file {
                file.path().cache_dir()
            } else {
                return Err(Error::WalletFileMissing);
            }
        };

        // create a state client
        self.state = Some(State::new(
            &cache_dir,
            status,
            http_state,
            http_prover,
            self.store.clone(),
        )?);

        Ok(())
    }

    /// Sync wallet state
    pub async fn sync(&self) -> Result<(), Error> {
        self.state()?.sync().await
    }

    /// Helper function to register for async-sync outside of connect
    pub async fn register_sync(&mut self) -> Result<(), Error> {
        match self.state.as_mut() {
            Some(w) => w.register_sync().await,
            None => Err(Error::Offline),
        }
    }

    /// Checks if the wallet has an active connection to the network
    pub async fn is_online(&self) -> bool {
        self.state.is_some()
    }

    /// Fetches the notes from the state.
    pub async fn get_all_notes(
        &self,
        addr: &Address,
    ) -> Result<Vec<DecodedNote>, Error> {
        if !addr.is_owned() {
            return Err(Error::Unauthorized);
        }

        let seed = self.store.get_seed();

        let index = addr.index()?;
        let vk = derive_phoenix_vk(seed, index);
        let pk = addr.pk()?;

        let live_notes = self.state()?.fetch_notes(pk)?;
        let spent_notes = self.state()?.cache().spent_notes(pk)?;

        let live_notes = live_notes
            .into_iter()
            .map(|data| (None, data.note, data.height));
        let spent_notes = spent_notes.into_iter().map(
            |(nullifier, NoteData { note, height })| {
                (Some(nullifier), note, height)
            },
        );
        let history = live_notes
            .chain(spent_notes)
            .map(|(nullified_by, note, block_height)| {
                let amount = note.value(Some(&vk)).unwrap();
                DecodedNote {
                    note,
                    amount,
                    block_height,
                    nullified_by,
                }
            })
            .collect();

        Ok(history)
    }

    /// Obtain balance information for a given address
    pub async fn get_phoenix_balance(
        &self,
        addr: &Address,
    ) -> Result<BalanceInfo, Error> {
        let state = self.state()?;
        // make sure we own this address
        if !addr.is_owned() {
            return Err(Error::Unauthorized);
        }

        let index = addr.index()?;
        let notes = state.fetch_notes(addr.pk()?)?;

        let seed = self.store.get_seed();

        Ok(phoenix_balance(
            &derive_phoenix_vk(seed, index),
            notes.iter(),
        ))
    }

    /// Get moonlight account balance
    pub fn get_moonlight_balance(&self, addr: &Address) -> Result<Dusk, Error> {
        let pk = addr.apk()?;
        let state = self.state()?;
        let account = state.fetch_account(pk)?;

        Ok(Dusk::from(account.balance))
    }

    /// Creates a new public address.
    /// The addresses generated are deterministic across sessions.
    pub fn new_address(&mut self) -> &Address {
        let seed = self.store.get_seed();
        let len = self.addresses.len();
        let pk = derive_phoenix_pk(seed, len as u8);
        let addr = Address::Phoenix {
            index: Some(len as u8),
            addr: pk,
        };

        self.addresses.push(addr);
        self.addresses.last().unwrap()
    }

    /// Default public address for this wallet
    pub fn default_address(&self) -> &Address {
        &self.addresses[0]
    }

    /// Addresses that have been generated by the user
    pub fn addresses(&self) -> &Vec<Address> {
        &self.addresses
    }

    /// Returns the phoenix secret-key for a given index
    pub(crate) fn phoenix_secret_key(&self, index: u8) -> PhoenixSecretKey {
        let seed = self.store.get_seed();
        derive_phoenix_sk(seed, index)
    }

    /// Returns the phoenix public-key for a given index
    pub fn phoenix_public_key(&self, index: u8) -> PhoenixPublicKey {
        let seed = self.store.get_seed();
        derive_phoenix_pk(seed, index)
    }

    /// Returns the bls secret-key for a given index
    pub(crate) fn bls_secret_key(&self, index: u8) -> BlsSecretKey {
        let seed = self.store.get_seed();
        derive_bls_sk(seed, index)
    }

    /// Returns the bls public-key for a given index
    pub fn bls_public_key(&self, index: u8) -> BlsPublicKey {
        let seed = self.store.get_seed();
        derive_bls_pk(seed, index)
    }

    /// Creates a generic moonlight transaction.
    #[allow(clippy::too_many_arguments)]
    pub fn moonlight_transaction(
        &self,
        from_addr: &Address,
        to_account: Option<BlsPublicKey>,
        transfer_value: Dusk,
        deposit: Dusk,
        gas: Gas,
        exec: Option<impl Into<TransactionData>>,
    ) -> Result<Transaction, Error> {
        // make sure we own the sender address
        if !from_addr.is_owned() {
            return Err(Error::Unauthorized);
        }

        // check gas limits
        if !gas.is_enough() {
            return Err(Error::NotEnoughGas);
        }

        let state = self.state()?;
        let deposit = *deposit;

        let from_index = from_addr.index()?;
        let mut from_sk = self.bls_secret_key(from_index);
        let from_account = self.bls_public_key(from_index);

        let account = state.fetch_account(&from_account)?;

        // technically this check is not necessary, but it's nice to not spam
        // the network with transactions that are unspendable.
        let nonce = account.nonce + 1;

        let chain_id = state.fetch_chain_id()?;

        let tx = moonlight(
            &from_sk,
            to_account,
            *transfer_value,
            deposit,
            gas.limit,
            gas.price,
            nonce,
            chain_id,
            exec,
        )?;

        from_sk.zeroize();

        state.prove_and_propagate(tx)
    }

    /// Executes a generic contract call, paying gas with phoenix notes
    pub async fn phoenix_execute(
        &self,
        sender: &Address,
        deposit: Dusk,
        gas: Gas,
        data: TransactionData,
    ) -> Result<Transaction, Error> {
        // make sure we own the sender address
        if !sender.is_owned() {
            return Err(Error::Unauthorized);
        }

        // check gas limits
        if !gas.is_enough() {
            return Err(Error::NotEnoughGas);
        }

        let state = self.state()?;
        let deposit = *deposit;

        let mut rng = StdRng::from_entropy();
        let sender_index = sender.index()?;
        let mut sender_sk = self.phoenix_secret_key(sender_index);
        // in a contract execution, the sender and receiver are the same
        let receiver_pk = sender.pk()?;

        let inputs = state
            .inputs(sender_index, deposit + gas.limit * gas.price)?
            .into_iter()
            .map(|(a, b, _)| (a, b))
            .collect();

        let root = state.fetch_root()?;
        let chain_id = state.fetch_chain_id()?;

        let tx = phoenix(
            &mut rng,
            &sender_sk,
            sender.pk()?,
            receiver_pk,
            inputs,
            root,
            0,
            true,
            deposit,
            gas.limit,
            gas.price,
            chain_id,
            Some(data),
            &Prover,
        )?;

        sender_sk.zeroize();

        state.prove_and_propagate(tx)
    }

    /// Transfers funds between phoenix-addresses
    pub async fn phoenix_transfer(
        &self,
        sender: &Address,
        rcvr: &Address,
        amt: Dusk,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        // make sure we own the sender address
        if !sender.is_owned() {
            return Err(Error::Unauthorized);
        }
        // make sure amount is positive
        if amt == 0 {
            return Err(Error::AmountIsZero);
        }
        // check gas limits
        if !gas.is_enough() {
            return Err(Error::NotEnoughGas);
        }

        let state = self.state()?;

        let mut rng = StdRng::from_entropy();
        let sender_index = sender.index()?;
        let amt = *amt;

        let mut sender_sk = self.phoenix_secret_key(sender_index);
        let change_pk = sender.pk()?;
        let reciever_pk = rcvr.pk()?;

        let inputs = state
            .inputs(sender_index, amt + gas.limit * gas.price)?
            .into_iter()
            .map(|(a, b, _)| (a, b))
            .collect();

        let root = state.fetch_root()?;
        let chain_id = state.fetch_chain_id()?;

        let tx = phoenix(
            &mut rng,
            &sender_sk,
            change_pk,
            reciever_pk,
            inputs,
            root,
            amt,
            true,
            0,
            gas.limit,
            gas.price,
            chain_id,
            None::<ContractCall>,
            &Prover,
        )?;

        sender_sk.zeroize();

        state.prove_and_propagate(tx)
    }

    /// Transfer through moonlight
    pub async fn moonlight_transfer(
        &self,
        sender: &Address,
        rcvr: &Address,
        amt: Dusk,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        // make sure we own the sender address
        if !sender.is_owned() {
            return Err(Error::Unauthorized);
        }
        // make sure amount is positive
        if amt == 0 {
            return Err(Error::AmountIsZero);
        }
        // check gas limits
        if !gas.is_enough() {
            return Err(Error::NotEnoughGas);
        }

        let sender = sender.index()?;

        let mut from_sk = self.bls_secret_key(sender);
        let apk = rcvr.apk()?;
        let from_pk = self.bls_public_key(sender);
        let amt = *amt;

        let state = self.state()?;
        let nonce = state.fetch_account(&from_pk)?.nonce + 1;
        let chain_id = state.fetch_chain_id()?;

        let tx = moonlight(
            &from_sk,
            Some(*apk),
            amt,
            0,
            gas.limit,
            gas.price,
            nonce,
            chain_id,
            None::<TransactionData>,
        )?;

        from_sk.zeroize();

        state.prove_and_propagate(tx)
    }

    /// Stakes Dusk using phoenix notes
    pub async fn phoenix_stake(
        &self,
        addr: &Address,
        amt: Dusk,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        // make sure we own the staking address
        if !addr.is_owned() {
            return Err(Error::Unauthorized);
        }
        // make sure amount is positive
        if amt == 0 {
            return Err(Error::AmountIsZero);
        }
        // check if the gas is enough
        if !gas.is_enough() {
            return Err(Error::NotEnoughGas);
        }

        let state = self.state()?;

        let mut rng = StdRng::from_entropy();
        let amt = *amt;
        let sender_index = addr.index()?;
        let mut sender_sk = self.phoenix_secret_key(sender_index);
        let mut stake_sk = self.bls_secret_key(sender_index);

        let nonce = state
            .fetch_stake(&AccountPublicKey::from(&stake_sk))?
            .map(|s| s.nonce)
            .unwrap_or(0)
            + 1;

        let inputs = state
            .inputs(sender_index, amt + gas.limit * gas.price)?
            .into_iter()
            .map(|(a, b, _)| (a, b))
            .collect();

        let root = state.fetch_root()?;
        let chain_id = state.fetch_chain_id()?;

        let stake = phoenix_stake(
            &mut rng, &sender_sk, &stake_sk, inputs, root, gas.limit,
            gas.price, chain_id, amt, nonce, &Prover,
        )?;

        sender_sk.zeroize();
        stake_sk.zeroize();

        state.prove_and_propagate(stake)
    }

    /// Stake via moonlight
    pub fn moonlight_stake(
        &self,
        addr: &Address,
        amt: Dusk,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        // make sure we own the staking address
        if !addr.is_owned() {
            return Err(Error::Unauthorized);
        }
        // make sure amount is positive
        if amt == 0 {
            return Err(Error::AmountIsZero);
        }
        // check if the gas is enough
        if !gas.is_enough() {
            return Err(Error::NotEnoughGas);
        }

        let state = self.state()?;
        let amt = *amt;
        let sender_index = addr.index()?;
        let mut stake_sk = self.bls_secret_key(sender_index);
        let pk = AccountPublicKey::from(&stake_sk);
        let chain_id = state.fetch_chain_id()?;
        let moonlight_current_nonce = state.fetch_account(&pk)?.nonce + 1;

        let nonce = state.fetch_stake(&pk)?.map(|s| s.nonce + 1).unwrap_or(0);

        let stake = moonlight_stake(
            &stake_sk,
            &stake_sk,
            amt,
            gas.limit,
            gas.price,
            moonlight_current_nonce,
            nonce,
            chain_id,
        )?;

        stake_sk.zeroize();

        state.prove_and_propagate(stake)
    }

    /// Obtains stake information for a given address
    pub async fn stake_info(
        &self,
        addr_idx: u8,
    ) -> Result<Option<StakeData>, Error> {
        self.state()?.fetch_stake(&self.bls_public_key(addr_idx))
    }

    /// Unstakes Dusk into phoenix notes
    pub async fn phoenix_unstake(
        &self,
        addr: &Address,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        // make sure we own the staking address
        if !addr.is_owned() {
            return Err(Error::Unauthorized);
        }

        let mut rng = StdRng::from_entropy();
        let index = addr.index()?;

        let state = self.state()?;

        let mut sender_sk = self.phoenix_secret_key(index);
        let mut stake_sk = self.bls_secret_key(index);

        let unstake_value = state
            .fetch_stake(&AccountPublicKey::from(&stake_sk))?
            .and_then(|s| s.amount)
            .map(|s| s.value)
            .unwrap_or(0);

        let inputs = state.inputs(index, gas.limit * gas.price)?;

        let root = state.fetch_root()?;
        let chain_id = state.fetch_chain_id()?;

        let unstake = phoenix_unstake(
            &mut rng,
            &sender_sk,
            &stake_sk,
            inputs,
            root,
            unstake_value,
            gas.limit,
            gas.price,
            chain_id,
            &Prover,
        )?;

        sender_sk.zeroize();
        stake_sk.zeroize();

        state.prove_and_propagate(unstake)
    }

    /// Unstakes Dusk through moonlight
    pub async fn moonlight_unstake(
        &self,
        addr: &Address,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        // make sure we own the staking address
        if !addr.is_owned() {
            return Err(Error::Unauthorized);
        }

        let mut rng = StdRng::from_entropy();
        let index = addr.index()?;
        let state = self.state()?;
        let mut stake_sk = self.bls_secret_key(index);

        let pk = AccountPublicKey::from(&stake_sk);

        let chain_id = state.fetch_chain_id()?;
        let account_nonce = state.fetch_account(&pk)?.nonce + 1;

        let unstake_value = state
            .fetch_stake(&pk)?
            .and_then(|s| s.amount)
            .map(|s| s.value)
            .unwrap_or(0);

        let unstake = moonlight_unstake(
            &mut rng,
            &stake_sk,
            &stake_sk,
            unstake_value,
            gas.price,
            gas.limit,
            account_nonce,
            chain_id,
        )?;

        stake_sk.zeroize();

        state.prove_and_propagate(unstake)
    }

    /// Withdraw accumulated staking reward for a given address
    pub async fn phoenix_stake_withdraw(
        &self,
        sender_addr: &Address,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        let state = self.state()?;
        // make sure we own the staking address
        if !sender_addr.is_owned() {
            return Err(Error::Unauthorized);
        }

        let mut rng = StdRng::from_entropy();
        let sender_index = sender_addr.index()?;

        let mut sender_sk = self.phoenix_secret_key(sender_index);
        let mut stake_sk = self.bls_secret_key(sender_index);

        let inputs = state.inputs(sender_index, gas.limit * gas.price)?;

        let root = state.fetch_root()?;
        let chain_id = state.fetch_chain_id()?;

        let reward_amount = state
            .fetch_stake(&AccountPublicKey::from(&stake_sk))?
            .map(|s| s.reward)
            .unwrap_or(0);

        let withdraw = phoenix_stake_reward(
            &mut rng,
            &sender_sk,
            &stake_sk,
            inputs,
            root,
            reward_amount,
            gas.limit,
            gas.price,
            chain_id,
            &Prover,
        )?;

        sender_sk.zeroize();
        stake_sk.zeroize();

        state.prove_and_propagate(withdraw)
    }

    /// Convert balance from phoenix to moonlight
    pub async fn phoenix_to_moonlight(
        &self,
        sender_addr: &Address,
        amt: Dusk,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        let mut rng = StdRng::from_entropy();
        let state = self.state()?;
        let sender_index = sender_addr.index()?;
        let amt = *amt;
        let inputs = state.inputs(sender_index, amt + gas.limit * gas.price)?;

        let root = state.fetch_root()?;
        let chain_id = state.fetch_chain_id()?;

        let mut sender_sk = self.phoenix_secret_key(sender_index);
        let mut stake_sk = self.bls_secret_key(sender_index);

        let convert = phoenix_to_moonlight(
            &mut rng, &sender_sk, &stake_sk, inputs, root, amt, gas.limit,
            gas.price, chain_id, &Prover,
        )?;

        sender_sk.zeroize();
        stake_sk.zeroize();

        state.prove_and_propagate(convert)
    }

    /// Convert balance from moonlight to phoenix
    pub async fn moonlight_to_phoenix(
        &self,
        sender_addr: &Address,
        amt: Dusk,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        let mut rng = StdRng::from_entropy();
        let state = self.state()?;
        let sender_index = sender_addr.index()?;
        let amt = *amt;

        let pk = self.bls_public_key(sender_index);

        let nonce = state.fetch_account(&pk)?.nonce + 1;
        let chain_id = state.fetch_chain_id()?;

        let mut sender_sk = self.phoenix_secret_key(sender_index);
        let mut stake_sk = self.bls_secret_key(sender_index);

        let convert = moonlight_to_phoenix(
            &mut rng, &stake_sk, &sender_sk, amt, gas.limit, gas.price, nonce,
            chain_id,
        )?;

        sender_sk.zeroize();
        stake_sk.zeroize();

        state.prove_and_propagate(convert)
    }

    /// Returns bls key pair for provisioner nodes
    pub fn provisioner_keys(
        &self,
        addr: &Address,
    ) -> Result<(BlsPublicKey, BlsSecretKey), Error> {
        // make sure we own the staking address
        if !addr.is_owned() {
            return Err(Error::Unauthorized);
        }

        let index = addr.index()?;
        let sk = self.bls_secret_key(index);
        let pk = self.bls_public_key(index);

        Ok((pk, sk))
    }

    /// Export bls key pair for provisioners in node-compatible format
    pub fn export_provisioner_keys(
        &self,
        addr: &Address,
        dir: &Path,
        filename: Option<String>,
        pwd: &[u8],
    ) -> Result<(PathBuf, PathBuf), Error> {
        // we're expecting a directory here
        if !dir.is_dir() {
            return Err(Error::NotDirectory);
        }

        // get our keys for this address
        let keys = self.provisioner_keys(addr)?;

        // set up the path
        let mut path = PathBuf::from(dir);
        path.push(filename.unwrap_or(addr.to_string()));

        // export public key to disk
        let bytes = keys.0.to_bytes();
        fs::write(path.with_extension("cpk"), bytes)?;

        // create node-compatible json structure
        let bls = BlsKeyPair {
            public_key_bls: keys.0.to_bytes(),
            secret_key_bls: keys.1.to_bytes(),
        };
        let json = serde_json::to_string(&bls)?;

        // encrypt data
        let mut bytes = json.as_bytes().to_vec();
        bytes = crate::crypto::encrypt(&bytes, pwd)?;

        // export key pair to disk
        fs::write(path.with_extension("keys"), bytes)?;

        Ok((path.with_extension("keys"), path.with_extension("cpk")))
    }

    /// Obtain the owned `Address` for a given address
    pub fn claim_as_address(&self, addr: Address) -> Result<&Address, Error> {
        self.addresses()
            .iter()
            .find(|&a| a == &addr)
            .ok_or(Error::AddressNotOwned)
    }

    /// Return the dat file version from memory or by reading the file
    /// In order to not read the file version more than once per execution
    pub fn get_file_version(&self) -> Result<DatFileVersion, Error> {
        if let Some(file_version) = self.file_version {
            Ok(file_version)
        } else if let Some(file) = &self.file {
            Ok(dat::read_file_version(file.path())?)
        } else {
            Err(Error::WalletFileMissing)
        }
    }
}

/// This structs represent a Note decoded enriched with useful chain information
pub struct DecodedNote {
    /// The phoenix note
    pub note: Note,
    /// The decoded amount
    pub amount: u64,
    /// The block height
    pub block_height: u64,
    /// Nullified by
    pub nullified_by: Option<BlsScalar>,
}

/// Bls key pair helper structure
#[derive(Serialize)]
struct BlsKeyPair {
    #[serde(with = "base64")]
    secret_key_bls: [u8; 32],
    #[serde(with = "base64")]
    public_key_bls: [u8; 96],
}

mod base64 {
    use serde::{Serialize, Serializer};

    pub fn serialize<S: Serializer>(v: &[u8], s: S) -> Result<S::Ok, S::Error> {
        let base64 = base64::encode(v);
        String::serialize(&base64, s)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use tempfile::tempdir;

    const TEST_ADDR: &str = "2w7fRQW23Jn9Bgm1GQW9eC2bD9U883dAwqP7HAr2F8g1syzPQaPYrxSyyVZ81yDS5C1rv9L8KjdPBsvYawSx3QCW";

    #[derive(Debug, Clone)]
    struct WalletFile {
        path: WalletPath,
        pwd: Vec<u8>,
    }

    impl SecureWalletFile for WalletFile {
        fn path(&self) -> &WalletPath {
            &self.path
        }

        fn pwd(&self) -> &[u8] {
            &self.pwd
        }
    }

    #[test]
    fn wallet_basics() -> Result<(), Box<dyn std::error::Error>> {
        // create a wallet from a mnemonic phrase
        let mut wallet: Wallet<WalletFile> = Wallet::new("uphold stove tennis fire menu three quick apple close guilt poem garlic volcano giggle comic")?;

        // check address generation
        let default_addr = wallet.default_address().clone();
        let other_addr = wallet.new_address();

        assert!(format!("{}", default_addr).eq(TEST_ADDR));
        assert_ne!(&default_addr, other_addr);
        assert_eq!(wallet.addresses.len(), 2);

        // create another wallet with different mnemonic
        let wallet: Wallet<WalletFile> = Wallet::new("demise monitor elegant cradle squeeze cheap parrot venture stereo humor scout denial action receive flat")?;

        // check addresses are different
        let addr = wallet.default_address();
        assert!(format!("{}", addr).ne(TEST_ADDR));

        // attempt to create a wallet from an invalid mnemonic
        let bad_wallet: Result<Wallet<WalletFile>, Error> =
            Wallet::new("good luck with life");
        assert!(bad_wallet.is_err());

        Ok(())
    }

    #[test]
    fn save_and_load() -> Result<(), Box<dyn std::error::Error>> {
        // prepare a tmp path
        let dir = tempdir()?;
        let path = dir.path().join("my_wallet.dat");
        let path = WalletPath::from(path);

        // we'll need a password too
        let pwd = blake3::hash("mypassword".as_bytes()).as_bytes().to_vec();

        // create and save
        let mut wallet: Wallet<WalletFile> = Wallet::new("uphold stove tennis fire menu three quick apple close guilt poem garlic volcano giggle comic")?;
        let file = WalletFile { path, pwd };
        wallet.save_to(file.clone())?;

        // load from file and check
        let loaded_wallet = Wallet::from_file(file)?;

        let original_addr = wallet.default_address();
        let loaded_addr = loaded_wallet.default_address();
        assert!(original_addr.eq(loaded_addr));

        Ok(())
    }
}
