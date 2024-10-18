// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod address;
mod file;
pub mod gas;

pub use address::{Address, Profile};
pub use file::{SecureWalletFile, WalletPath};

use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};

use bip39::{Language, Mnemonic, Seed};
use dusk_bytes::Serializable;
use rand::rngs::StdRng;
use rand::SeedableRng;
use rues::RuesHttpClient;
use serde::Serialize;
use zeroize::Zeroize;

use execution_core::{
    signatures::bls::{PublicKey as BlsPublicKey, SecretKey as BlsSecretKey},
    transfer::{data::TransactionData, phoenix::NoteLeaf, Transaction},
    CONTRACT_ID_BYTES,
};
use wallet_core::{
    phoenix_balance,
    prelude::keys::{
        derive_bls_pk, derive_bls_sk, derive_phoenix_pk, derive_phoenix_sk,
        derive_phoenix_vk,
    },
    transaction::{
        moonlight, moonlight_deployment, moonlight_stake,
        moonlight_stake_reward, moonlight_to_phoenix, moonlight_unstake,
        phoenix, phoenix_deployment, phoenix_stake, phoenix_stake_reward,
        phoenix_to_moonlight, phoenix_unstake,
    },
    BalanceInfo,
};

use super::*;
use crate::{
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
/// The user can generate as many [`Profile`] as needed without an active
/// connection to the network by calling [`Wallet::new_address`] repeatedly.
///
/// A wallet must connect to the network using a [`RuskEndpoint`] in order to be
/// able to perform common operations such as checking balance, transfernig
/// funds, or staking Dusk.
pub struct Wallet<F: SecureWalletFile + Debug> {
    profiles: Vec<Profile>,
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
            let seed_bytes = seed.as_bytes().try_into().unwrap();

            // Generate the default address at index 0
            let profiles = vec![Profile {
                shielded_addr: derive_phoenix_pk(&seed_bytes, 0),
                public_addr: derive_bls_pk(&seed_bytes, 0),
            }];

            // return new wallet instance
            Ok(Wallet {
                profiles,
                state: None,
                store: LocalStore::from(seed_bytes),
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
            // Generate the default address at index 0
            let profiles = vec![Profile {
                shielded_addr: derive_phoenix_pk(&seed, 0),
                public_addr: derive_bls_pk(&seed, 0),
            }];

            // return the store
            return Ok(Self {
                profiles,
                store: LocalStore::from(seed),
                state: None,
                file: Some(file),
                file_version: Some(DatFileVersion::Legacy),
            });
        }

        let profiles: Vec<_> = (0..address_count)
            .map(|i| Profile {
                shielded_addr: derive_phoenix_pk(&seed, i),
                public_addr: derive_bls_pk(&seed, i),
            })
            .collect();

        // create and return
        Ok(Self {
            profiles,
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

                payload.push(self.profiles.len() as u8);

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
        profile_idx: u8,
    ) -> Result<Vec<DecodedNote>, Error> {
        let vk = self.derive_phoenix_vk(profile_idx);
        let pk = self.shielded_addr(profile_idx)?;

        let live_notes = self.state()?.fetch_notes(pk)?;
        let spent_notes = self.state()?.cache().spent_notes(pk)?;

        let live_notes = live_notes
            .into_iter()
            .map(|data| (None, data.note, data.block_height));
        let spent_notes = spent_notes.into_iter().map(
            |(nullifier, NoteLeaf { note, block_height })| {
                (Some(nullifier), note, block_height)
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

    /// Get the Phoenix balance
    pub async fn get_phoenix_balance(
        &self,
        profile_idx: u8,
    ) -> Result<BalanceInfo, Error> {
        let state = self.state()?;

        let notes = state.fetch_notes(self.shielded_addr(profile_idx)?)?;

        Ok(phoenix_balance(
            &self.derive_phoenix_vk(profile_idx),
            notes.iter(),
        ))
    }

    /// Get Moonlight account balance
    pub async fn get_moonlight_balance(
        &self,
        profile_idx: u8,
    ) -> Result<Dusk, Error> {
        let pk = self.public_addr(profile_idx)?;
        let state = self.state()?;
        let account = state.fetch_account(pk).await?;

        Ok(Dusk::from(account.balance))
    }

    /// Pushes a new entry to the internal profiles vector and returns its
    /// index.
    pub fn add_profile(&mut self) -> u8 {
        let seed = self.store.get_seed();
        let index = self.profiles.len() as u8;
        let addr = Profile {
            shielded_addr: derive_phoenix_pk(seed, index),
            public_addr: derive_bls_pk(seed, index),
        };

        self.profiles.push(addr);

        index
    }

    /// Returns the default shielded address for this wallet
    pub fn default_shielded_addr(&self) -> &PhoenixPublicKey {
        self.shielded_addr(0)
            .expect("there to be an address at index 0")
    }

    /// Returns the default public account address for this wallet
    pub fn default_public_addr(&self) -> &BlsPublicKey {
        self.public_addr(0)
            .expect("there to be an address at index 0")
    }

    /// Returns the profiles that have been generated by the user
    pub fn profiles(&self) -> &Vec<Profile> {
        &self.profiles
    }

    /// Returns the Phoenix secret-key for a given index
    pub(crate) fn derive_phoenix_sk(&self, index: u8) -> PhoenixSecretKey {
        let seed = self.store.get_seed();
        derive_phoenix_sk(seed, index)
    }

    /// Returns the Phoenix view-key for a given index
    pub(crate) fn derive_phoenix_vk(&self, index: u8) -> PhoenixViewKey {
        let seed = self.store.get_seed();
        derive_phoenix_vk(seed, index)
    }

    /// Returns the shielded address for a given index.
    ///
    /// # Errors
    /// This will error if the wallet doesn't have a profile stored for the
    /// given index.
    pub fn shielded_addr(&self, index: u8) -> Result<&PhoenixPublicKey, Error> {
        let index = usize::from(index);
        if index >= self.profiles.len() {
            return Err(Error::Unauthorized);
        }

        Ok(&self.profiles()[index].shielded_addr)
    }

    /// Returns the BLS secret-key for a given index
    pub(crate) fn derive_bls_sk(&self, index: u8) -> BlsSecretKey {
        let seed = self.store.get_seed();
        derive_bls_sk(seed, index)
    }

    /// Returns the public account address for a given index.
    ///
    /// # Errors
    /// This will error if the wallet doesn't have a profile stored for the
    /// given index.
    pub fn public_addr(&self, index: u8) -> Result<&BlsPublicKey, Error> {
        let index = usize::from(index);
        if index >= self.profiles.len() {
            return Err(Error::Unauthorized);
        }

        Ok(&self.profiles()[index].public_addr)
    }

    /// Executes a generic contract call, paying gas from a public account.
    #[allow(clippy::too_many_arguments)]
    pub async fn moonlight_execute(
        &self,
        sender_idx: u8,
        transfer_value: Dusk,
        deposit: Dusk,
        gas: Gas,
        exec: Option<impl Into<TransactionData>>,
    ) -> Result<Transaction, Error> {
        // check gas limits
        if !gas.is_enough() {
            return Err(Error::NotEnoughGas);
        }

        let state = self.state()?;
        let deposit = *deposit;

        let mut sender_sk = self.derive_bls_sk(sender_idx);
        let sender = self.public_addr(sender_idx)?;

        let account = state.fetch_account(sender).await?;

        // technically this check is not necessary, but it's nice to not spam
        // the network with transactions that are unspendable.
        let nonce = account.nonce + 1;

        let chain_id = state.fetch_chain_id().await?;

        let tx = moonlight(
            &sender_sk,
            None,
            *transfer_value,
            deposit,
            gas.limit,
            gas.price,
            nonce,
            chain_id,
            exec,
        )?;

        sender_sk.zeroize();

        state.propagate(tx).await
    }

    /// Executes a generic contract call, paying gas with a shielded address.
    pub async fn phoenix_execute(
        &self,
        sender_idx: u8,
        deposit: Dusk,
        gas: Gas,
        data: TransactionData,
    ) -> Result<Transaction, Error> {
        // check gas limits
        if !gas.is_enough() {
            return Err(Error::NotEnoughGas);
        }

        let state = self.state()?;
        let deposit = *deposit;

        let mut rng = StdRng::from_entropy();
        let mut sender_sk = self.derive_phoenix_sk(sender_idx);
        // in a contract execution or deployment, the sender and receiver are
        // the same
        let receiver_pk = self.shielded_addr(sender_idx)?;

        let inputs = state
            .inputs(sender_idx, deposit + gas.limit * gas.price)
            .await?
            .into_iter()
            .map(|(a, b, _)| (a, b))
            .collect();

        let root = state.fetch_root().await?;
        let chain_id = state.fetch_chain_id().await?;

        let tx = phoenix(
            &mut rng,
            &sender_sk,
            self.shielded_addr(sender_idx)?,
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

        let tx = state.prove(tx).await?;
        state.propagate(tx).await
    }

    /// Transfers funds between shielded addresses.
    pub async fn phoenix_transfer(
        &self,
        sender_idx: u8,
        receiver_pk: &PhoenixPublicKey,
        memo: Option<String>,
        amt: Dusk,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        // make sure amount is positive
        if amt == 0 && memo.is_none() {
            return Err(Error::AmountIsZero);
        }
        // check gas limits
        if !gas.is_enough() {
            return Err(Error::NotEnoughGas);
        }

        let state = self.state()?;

        let mut rng = StdRng::from_entropy();
        let amt = *amt;

        let mut sender_sk = self.derive_phoenix_sk(sender_idx);
        let refund_pk = self.shielded_addr(sender_idx)?;

        let inputs = state
            .inputs(sender_idx, amt + gas.limit * gas.price)
            .await?
            .into_iter()
            .map(|(a, b, _)| (a, b))
            .collect();

        let root = state.fetch_root().await?;
        let chain_id = state.fetch_chain_id().await?;

        let tx = phoenix(
            &mut rng,
            &sender_sk,
            refund_pk,
            receiver_pk,
            inputs,
            root,
            amt,
            true,
            0,
            gas.limit,
            gas.price,
            chain_id,
            memo,
            &Prover,
        )?;

        sender_sk.zeroize();

        let tx = state.prove(tx).await?;
        state.propagate(tx).await
    }

    /// Transfers funds between public accounts.
    pub async fn moonlight_transfer(
        &self,
        sender_idx: u8,
        rcvr: &BlsPublicKey,
        memo: Option<String>,
        amt: Dusk,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        // make sure amount is positive
        if amt == 0 && memo.is_none() {
            return Err(Error::AmountIsZero);
        }
        // check gas limits
        if !gas.is_enough() {
            return Err(Error::NotEnoughGas);
        }

        let mut sender_sk = self.derive_bls_sk(sender_idx);
        let sender_pk = self.public_addr(sender_idx)?;
        let amt = *amt;

        let state = self.state()?;
        let nonce = state.fetch_account(sender_pk).await?.nonce + 1;
        let chain_id = state.fetch_chain_id().await?;

        let tx = moonlight(
            &sender_sk,
            Some(*rcvr),
            amt,
            0,
            gas.limit,
            gas.price,
            nonce,
            chain_id,
            memo,
        )?;

        sender_sk.zeroize();

        state.propagate(tx).await
    }

    /// Stakes Dusk using shielded notes.
    pub async fn phoenix_stake(
        &self,
        profile_idx: u8,
        amt: Dusk,
        gas: Gas,
    ) -> Result<Transaction, Error> {
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
        let mut sender_sk = self.derive_phoenix_sk(profile_idx);
        let mut stake_sk = self.derive_bls_sk(profile_idx);

        let stake_pk = self.public_addr(profile_idx)?;
        let current_stake = state.fetch_stake(stake_pk).await?;
        if let Some(stake) = current_stake {
            if stake.amount.is_some() {
                return Err(Error::AlreadyStaked);
            }
        }

        let nonce = current_stake.map(|s| s.nonce).unwrap_or(0) + 1;

        let inputs = state
            .inputs(profile_idx, amt + gas.limit * gas.price)
            .await?
            .into_iter()
            .map(|(a, b, _)| (a, b))
            .collect();

        let root = state.fetch_root().await?;
        let chain_id = state.fetch_chain_id().await?;

        let stake = phoenix_stake(
            &mut rng, &sender_sk, &stake_sk, inputs, root, gas.limit,
            gas.price, chain_id, amt, nonce, &Prover,
        )?;

        sender_sk.zeroize();
        stake_sk.zeroize();

        let stake = state.prove(stake).await?;
        state.propagate(stake).await
    }

    /// Stakes Dusk using a public account.
    pub async fn moonlight_stake(
        &self,
        profile_idx: u8,
        amt: Dusk,
        gas: Gas,
    ) -> Result<Transaction, Error> {
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
        let mut stake_sk = self.derive_bls_sk(profile_idx);
        let stake_pk = self.public_addr(profile_idx)?;
        let chain_id = state.fetch_chain_id().await?;
        let moonlight_current_nonce =
            state.fetch_account(stake_pk).await?.nonce + 1;

        let current_stake = state.fetch_stake(stake_pk).await?;
        if let Some(stake) = current_stake {
            if stake.amount.is_some() {
                return Err(Error::AlreadyStaked);
            }
        }

        let nonce = current_stake.map(|s| s.nonce).unwrap_or(0) + 1;

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

        state.propagate(stake).await
    }

    /// Obtains stake information for a given address.
    pub async fn stake_info(
        &self,
        profile_idx: u8,
    ) -> Result<Option<StakeData>, Error> {
        self.state()?
            .fetch_stake(self.public_addr(profile_idx)?)
            .await
    }

    /// Unstakes Dusk into shielded notes.
    pub async fn phoenix_unstake(
        &self,
        profile_idx: u8,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        let mut rng = StdRng::from_entropy();

        let state = self.state()?;

        let mut sender_sk = self.derive_phoenix_sk(profile_idx);
        let mut stake_sk = self.derive_bls_sk(profile_idx);

        let unstake_value = state
            .fetch_stake(&BlsPublicKey::from(&stake_sk))
            .await?
            .and_then(|s| s.amount)
            .map(|s| s.total_funds())
            .unwrap_or_default();

        if unstake_value == 0 {
            return Err(Error::NotStaked);
        }

        let inputs = state.inputs(profile_idx, gas.limit * gas.price).await?;

        let root = state.fetch_root().await?;
        let chain_id = state.fetch_chain_id().await?;

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

        let unstake = state.prove(unstake).await?;
        state.propagate(unstake).await
    }

    /// Unstakes Dusk onto a public account.
    pub async fn moonlight_unstake(
        &self,
        profile_idx: u8,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        let mut rng = StdRng::from_entropy();
        let state = self.state()?;
        let mut stake_sk = self.derive_bls_sk(profile_idx);

        let pk = self.public_addr(profile_idx)?;

        let chain_id = state.fetch_chain_id().await?;
        let account_nonce = state.fetch_account(pk).await?.nonce + 1;

        let unstake_value = state
            .fetch_stake(pk)
            .await?
            .and_then(|s| s.amount)
            .map(|s| s.total_funds())
            .unwrap_or_default();

        if unstake_value == 0 {
            return Err(Error::NotStaked);
        }

        let unstake = moonlight_unstake(
            &mut rng,
            &stake_sk,
            &stake_sk,
            unstake_value,
            gas.limit,
            gas.price,
            account_nonce,
            chain_id,
        )?;

        stake_sk.zeroize();

        state.propagate(unstake).await
    }

    /// Withdraws accumulated staking to a shielded address.
    pub async fn phoenix_stake_withdraw(
        &self,
        sender_idx: u8,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        let state = self.state()?;
        let mut rng = StdRng::from_entropy();

        let mut sender_sk = self.derive_phoenix_sk(sender_idx);
        let mut stake_sk = self.derive_bls_sk(sender_idx);

        let inputs = state.inputs(sender_idx, gas.limit * gas.price).await?;

        let root = state.fetch_root().await?;
        let chain_id = state.fetch_chain_id().await?;

        let reward_amount = state
            .fetch_stake(&BlsPublicKey::from(&stake_sk))
            .await?
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

        let withdraw = state.prove(withdraw).await?;
        state.propagate(withdraw).await
    }

    /// Converts Dusk from a shielded address to a public account.
    pub async fn phoenix_to_moonlight(
        &self,
        profile_idx: u8,
        amt: Dusk,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        let mut rng = StdRng::from_entropy();
        let state = self.state()?;
        let amt = *amt;
        let inputs = state
            .inputs(profile_idx, amt + gas.limit * gas.price)
            .await?;

        let root = state.fetch_root().await?;
        let chain_id = state.fetch_chain_id().await?;

        let mut phoenix_sk = self.derive_phoenix_sk(profile_idx);
        let mut moonlight_sk = self.derive_bls_sk(profile_idx);

        let convert = phoenix_to_moonlight(
            &mut rng,
            &phoenix_sk,
            &moonlight_sk,
            inputs,
            root,
            amt,
            gas.limit,
            gas.price,
            chain_id,
            &Prover,
        )?;

        phoenix_sk.zeroize();
        moonlight_sk.zeroize();

        let convert = state.prove(convert).await?;
        state.propagate(convert).await
    }

    /// Converts Dusk from a public account to a shielded address.
    pub async fn moonlight_to_phoenix(
        &self,
        profile_idx: u8,
        amt: Dusk,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        let mut rng = StdRng::from_entropy();
        let state = self.state()?;

        let moonlight_pk = self.public_addr(profile_idx)?;

        let nonce = state.fetch_account(moonlight_pk).await?.nonce + 1;
        let chain_id = state.fetch_chain_id().await?;

        let mut phoenix_sk = self.derive_phoenix_sk(profile_idx);
        let mut moonlight_sk = self.derive_bls_sk(profile_idx);

        let convert = moonlight_to_phoenix(
            &mut rng,
            &moonlight_sk,
            &phoenix_sk,
            *amt,
            gas.limit,
            gas.price,
            nonce,
            chain_id,
        )?;

        phoenix_sk.zeroize();
        moonlight_sk.zeroize();

        state.propagate(convert).await
    }

    /// Withdraws accumulated staking reward to a public account.
    pub async fn moonlight_stake_withdraw(
        &self,
        sender_idx: u8,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        let mut rng = StdRng::from_entropy();
        let state = self.state()?;

        let pk = self.public_addr(sender_idx)?;
        let nonce = state.fetch_account(pk).await?.nonce + 1;
        let chain_id = state.fetch_chain_id().await?;
        let stake_info = state.fetch_stake(pk).await?;
        let reward = stake_info.map(|s| s.reward).ok_or(Error::NoReward)?;
        let reward = Dusk::from(reward);

        let mut sender_sk = self.derive_bls_sk(sender_idx);

        let withdraw = moonlight_stake_reward(
            &mut rng, &sender_sk, &sender_sk, *reward, gas.limit, gas.price,
            nonce, chain_id,
        )?;

        sender_sk.zeroize();

        state.propagate(withdraw).await
    }

    /// Deploys a contract using a public account to pay gas.
    pub async fn moonlight_deploy(
        &self,
        sender_idx: u8,
        bytes_code: Vec<u8>,
        init_args: Vec<u8>,
        deploy_nonce: u64,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        let state = self.state()?;

        let pk = self.public_addr(sender_idx)?;
        let moonlight_nonce = state.fetch_account(pk).await?.nonce + 1;
        let chain_id = state.fetch_chain_id().await?;

        let mut sender_sk = self.derive_bls_sk(sender_idx);

        let deploy = moonlight_deployment(
            &sender_sk,
            bytes_code,
            pk,
            init_args,
            gas.limit,
            gas.price,
            moonlight_nonce,
            deploy_nonce,
            chain_id,
        )?;

        sender_sk.zeroize();

        state.propagate(deploy).await
    }

    /// Deploys a contract using shielded notes to pay gas.
    pub async fn phoenix_deploy(
        &self,
        sender_idx: u8,
        bytes_code: Vec<u8>,
        init_args: Vec<u8>,
        deploy_nonce: u64,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        let mut rng = StdRng::from_entropy();
        let state = self.state()?;

        let chain_id = state.fetch_chain_id().await?;
        let root = state.fetch_root().await?;

        let inputs = state.inputs(sender_idx, gas.limit * gas.price).await?;

        let mut sender_sk = self.derive_phoenix_sk(sender_idx);
        let owner_pk = self.public_addr(sender_idx)?;

        let deploy = phoenix_deployment(
            &mut rng,
            &sender_sk,
            inputs,
            root,
            bytes_code,
            owner_pk,
            init_args,
            deploy_nonce,
            gas.limit,
            gas.price,
            chain_id,
            &Prover,
        )?;

        sender_sk.zeroize();

        let deploy = state.prove(deploy).await?;
        state.propagate(deploy).await
    }

    /// Returns BLS key-pair for provisioner nodes
    pub fn provisioner_keys(
        &self,
        index: u8,
    ) -> Result<(BlsPublicKey, BlsSecretKey), Error> {
        let pk = *self.public_addr(index)?;
        let sk = self.derive_bls_sk(index);

        // make sure our internal addresses are not corrupted
        if pk != BlsPublicKey::from(&sk) {
            return Err(Error::Unauthorized);
        }

        Ok((pk, sk))
    }

    /// Exports BLS key-pair for provisioners in node-compatible format
    pub fn export_provisioner_keys(
        &self,
        profile_idx: u8,
        dir: &Path,
        filename: Option<String>,
        pwd: &[u8],
    ) -> Result<(PathBuf, PathBuf), Error> {
        // we're expecting a directory here
        if !dir.is_dir() {
            return Err(Error::NotDirectory);
        }

        // get our keys for this address
        let keys = self.provisioner_keys(profile_idx)?;

        // set up the path
        let mut path = PathBuf::from(dir);
        path.push(filename.unwrap_or(profile_idx.to_string()));

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

        // export key-pair to disk
        fs::write(path.with_extension("keys"), bytes)?;

        Ok((path.with_extension("keys"), path.with_extension("cpk")))
    }

    /// Return the index of the address passed, returns an error if the address
    /// is not in the wallet profiles.
    pub fn find_index(&self, addr: &Address) -> Result<u8, Error> {
        // check if the key is stored in our profiles, return its index if
        // found
        for (index, profile) in self.profiles().iter().enumerate() {
            if match addr {
                Address::Shielded { addr } => *addr == profile.shielded_addr,
                Address::Public { addr } => *addr == profile.public_addr,
            } {
                return Ok(index as u8);
            }
        }

        // return an error otherwise
        Err(Error::Unauthorized)
    }

    /// Generate a contract id given bytes and nonce
    pub fn get_contract_id(
        &self,
        profile_idx: u8,
        bytes: Vec<u8>,
        nonce: u64,
    ) -> Result<[u8; CONTRACT_ID_BYTES], Error> {
        let owner = self.public_addr(profile_idx)?.to_bytes();

        let mut hasher = blake2b_simd::Params::new()
            .hash_length(CONTRACT_ID_BYTES)
            .to_state();
        hasher.update(bytes.as_ref());
        hasher.update(&nonce.to_le_bytes()[..]);
        hasher.update(owner.as_ref());
        hasher
            .finalize()
            .as_bytes()
            .try_into()
            .map_err(|_| Error::InvalidContractId)
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

    /// Check if the wallet is synced
    pub async fn is_synced(&mut self) -> Result<bool, Error> {
        let state = self.state()?;
        let db_pos = state.cache().last_pos()?.unwrap_or(0);
        let network_last_pos = state.fetch_num_notes().await? - 1;

        Ok(network_last_pos == db_pos)
    }

    /// Close the wallet and zeroize the seed
    pub fn close(&mut self) {
        self.store.inner_mut().zeroize();

        // close the state if exists
        if let Some(x) = &mut self.state {
            x.close();
        }
    }
}

/// This structs represent a Note decoded enriched with useful chain information
pub struct DecodedNote {
    /// The Phoenix note
    pub note: Note,
    /// The decoded amount
    pub amount: u64,
    /// The block height
    pub block_height: u64,
    /// Nullified by
    pub nullified_by: Option<BlsScalar>,
}

/// BLS key-pair helper structure
#[derive(Serialize)]
struct BlsKeyPair {
    #[serde(with = "base64")]
    secret_key_bls: [u8; 32],
    #[serde(with = "base64")]
    public_key_bls: [u8; 96],
}

mod base64 {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
    use serde::{Serialize, Serializer};

    pub fn serialize<S: Serializer>(v: &[u8], s: S) -> Result<S::Ok, S::Error> {
        let base64 = BASE64.encode(v);
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

    fn default_phoenix_address(wallet: &Wallet<WalletFile>) -> Address {
        Address::Shielded {
            addr: *wallet
                .shielded_addr(0)
                .expect("There to be a key at the index"),
        }
    }

    #[test]
    fn wallet_basics() -> Result<(), Box<dyn std::error::Error>> {
        // create a wallet from a mnemonic phrase
        let mut wallet: Wallet<WalletFile> = Wallet::new("uphold stove tennis fire menu three quick apple close guilt poem garlic volcano giggle comic")?;

        // check address generation
        let default_addr = default_phoenix_address(&wallet);
        let other_addr_idx = wallet.add_profile();
        let other_addr = Address::Shielded {
            addr: *wallet.shielded_addr(other_addr_idx)?,
        };

        assert!(format!("{default_addr}").eq(TEST_ADDR));
        assert_ne!(default_addr, other_addr);
        assert_eq!(wallet.profiles.len(), 2);

        // create another wallet with different mnemonic
        let wallet: Wallet<WalletFile> = Wallet::new("demise monitor elegant cradle squeeze cheap parrot venture stereo humor scout denial action receive flat")?;

        // check addresses are different
        let addr = default_phoenix_address(&wallet);
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

        let original_addr = default_phoenix_address(&wallet);
        let loaded_addr = default_phoenix_address(&loaded_wallet);
        assert!(original_addr.eq(&loaded_addr));

        Ok(())
    }
}
