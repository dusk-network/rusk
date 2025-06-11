// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod address;
mod file;
mod transaction;

pub use address::{Address, Profile};
#[allow(clippy::module_name_repetitions)]
pub use file::{Secure as SecureWalletFile, WalletPath};

use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};

use bip39::{Language, Mnemonic, Seed};
use dusk_bytes::Serializable;
use dusk_core::abi::CONTRACT_ID_BYTES;
use dusk_core::signatures::bls::{
    PublicKey as BlsPublicKey, SecretKey as BlsSecretKey,
};
use dusk_core::stake::StakeData;
use dusk_core::transfer::phoenix::{
    Note, NoteLeaf, PublicKey as PhoenixPublicKey,
    SecretKey as PhoenixSecretKey, ViewKey as PhoenixViewKey,
};
use dusk_core::BlsScalar;
use wallet_core::prelude::keys::{
    derive_bls_pk, derive_bls_sk, derive_phoenix_pk, derive_phoenix_sk,
    derive_phoenix_vk,
};
use wallet_core::{phoenix_balance, BalanceInfo};
use zeroize::Zeroize;

use crate::clients::State;
use crate::crypto::encrypt;
use crate::currency::Dusk;
use crate::dat::{
    self, version_bytes, FileVersion as DatFileVersion, FILE_TYPE,
    LATEST_VERSION, MAGIC, RESERVED,
};
use crate::gas::MempoolGasPrices;
use crate::rues::HttpClient as RuesHttpClient;
use crate::store::LocalStore;
use crate::Error;

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
    ///
    /// # Errors
    /// This method will error if the provided phrase is not a valid mnemonic.
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
            let seed_bytes = seed
                .as_bytes()
                .try_into()
                .map_err(|_| Error::InvalidMnemonicPhrase)?;

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
    ///
    /// # Errors
    /// This method will error if the provided wallet-file is invalid.
    pub fn from_file(file: F) -> Result<Self, Error> {
        let path = file.path();
        let key = file.aes_key();

        // make sure file exists
        let pb = path.inner().clone();
        if !pb.is_file() {
            return Err(Error::WalletFileMissing);
        }

        // attempt to load and decode wallet
        let bytes = fs::read(&pb)?;

        let file_version = dat::check_version(bytes.get(0..12))?;

        let (seed, address_count) =
            dat::get_seed_and_address(file_version, bytes, key, file.iv())?;

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
    ///
    /// # Errors
    /// This method will error if the wallet-file is missing or if the file
    /// encryption fails.
    ///
    /// # Panics
    /// This method will panic if there is a wallet-file, but the iv or salt for
    /// the wallet encryption is missing.
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
                let salt = f.salt().expect("Couldn't find the salt");
                let iv = f.iv().expect("Couldn't find the IV");
                let mut payload = seed.to_vec();

                // we know that `len < MAX_PROFILES <= u8::MAX`, so casting to
                // u8 is safe here
                #[allow(clippy::cast_possible_truncation)]
                payload.push(self.profiles.len() as u8);

                // encrypt the payload
                let encrypted_payload = encrypt(&payload, f.aes_key(), iv)?;

                let mut content = Vec::with_capacity(
                    header.len()
                        + salt.len()
                        + iv.len()
                        + encrypted_payload.len(),
                );

                content.extend_from_slice(&header);
                content.extend_from_slice(salt);
                content.extend_from_slice(iv);
                content.extend_from_slice(&encrypted_payload);

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
    ///
    /// # Errors
    /// This method will error if the file encryption fails.
    pub fn save_to(&mut self, file: F) -> Result<(), Error> {
        // set our new file and save
        self.file = Some(file);
        self.save()
    }

    /// Access the inner state of the wallet
    ///
    /// # Errors
    /// This method will error if the wallet cannot connect to the network.
    pub fn state(&self) -> Result<&State, Error> {
        if let Some(state) = self.state.as_ref() {
            Ok(state)
        } else {
            Err(Error::Offline)
        }
    }

    /// Connect the wallet to the network providing a callback for status
    /// updates
    ///
    /// # Errors
    /// This method will error if either the `rusk_addr` or `prov_addr` is
    /// invalid or if the wallet-file is missing.
    pub async fn connect_with_status<S: Into<String>>(
        &mut self,
        rusk_addr: S,
        prov_addr: S,
        archiver_addr: S,
        status: fn(&str),
    ) -> Result<(), Error> {
        // attempt connection
        let http_state = RuesHttpClient::new(rusk_addr)?;
        let http_prover = RuesHttpClient::new(prov_addr)?;
        let http_archiver = RuesHttpClient::new(archiver_addr)?;

        let state_status = http_state.check_connection().await;
        let prover_status = http_prover.check_connection().await;
        let archiver_status = http_archiver.check_connection().await;

        match (&state_status, prover_status, archiver_status) {
            (Err(e),_, _)=> println!("Connection to Rusk Failed, some operations won't be available: {e}"),
            (_,Err(e), _)=> println!("Connection to Prover Failed, some operations won't be available: {e}"),
            (_, _, Err(e)) => println!("Connection to Archiver Failed, some operations won't be available: {e}"),
            _=> {},
        }

        let cache_dir = self.cache_path()?;

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
    ///
    /// # Errors
    /// This method will error if the wallet cannot connect to the network.
    pub async fn sync(&self) -> Result<(), Error> {
        self.state()?.sync().await
    }

    /// Helper function to register for async-sync outside of connect
    ///
    /// # Errors
    /// This method will error if the wallet is not connected to the network.
    pub fn register_sync(&mut self) -> Result<(), Error> {
        match self.state.as_mut() {
            Some(w) => {
                w.register_sync();
                Ok(())
            }
            None => Err(Error::Offline),
        }
    }

    /// Checks if the wallet has an active connection to the network
    pub async fn is_online(&self) -> bool {
        if let Some(state) = &self.state {
            state.check_connection().await
        } else {
            false
        }
    }

    /// Fetches the notes from the state.
    ///
    /// # Errors
    /// This method will error if the wallet is not connected to the network,
    /// if there is no profile stored for the given `profile_idx`, or if the
    /// stored notes are corrupted.
    pub fn get_all_notes(
        &self,
        profile_idx: u8,
    ) -> Result<Vec<DecodedNote>, Error> {
        let vk = self.derive_phoenix_vk(profile_idx);
        let pk = self.shielded_key(profile_idx)?;

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
            .flat_map(
                |(nullified_by, note, block_height)| -> Result<_, Error> {
                    let amount = note.value(Some(&vk));
                    if let Ok(amount) = amount {
                        Ok(DecodedNote {
                            note,
                            amount,
                            block_height,
                            nullified_by,
                        })
                    } else {
                        Err(Error::WrongViewKey)
                    }
                },
            )
            .collect();

        Ok(history)
    }

    /// Get the Phoenix balance
    ///
    /// # Errors
    /// This method will error if the wallet is not connected to the network or
    /// if there is no profile stored for the given `profile_idx`.
    pub async fn get_phoenix_balance(
        &self,
        profile_idx: u8,
    ) -> Result<BalanceInfo, Error> {
        self.sync().await?;

        let notes =
            self.state()?.fetch_notes(self.shielded_key(profile_idx)?)?;

        Ok(phoenix_balance(
            &self.derive_phoenix_vk(profile_idx),
            notes.iter(),
        ))
    }

    /// Get Moonlight account balance
    ///
    /// # Errors
    /// This method will error if the wallet is not connected to the network or
    /// if there is no profile stored for the given `profile_idx`.
    pub async fn get_moonlight_balance(
        &self,
        profile_idx: u8,
    ) -> Result<Dusk, Error> {
        let pk = self.public_key(profile_idx)?;
        let state = self.state()?;
        let account = state.fetch_account(pk).await?;

        Ok(Dusk::from(account.balance))
    }

    /// Pushes a new entry to the internal profiles vector and returns its
    /// index.
    pub fn add_profile(&mut self) -> u8 {
        let seed = self.store.get_seed();
        // we know that `len < MAX_PROFILES <= u8::MAX`, so casting to
        // u8 is safe here
        #[allow(clippy::cast_possible_truncation)]
        let index = self.profiles.len() as u8;
        let addr = Profile {
            shielded_addr: derive_phoenix_pk(seed, index),
            public_addr: derive_bls_pk(seed, index),
        };

        self.profiles.push(addr);

        index
    }

    /// Returns the default address for this wallet
    pub fn default_address(&self) -> Address {
        // TODO: let the user specify the default address using conf
        self.default_public_address()
    }

    /// Returns the default shielded account address for this wallet
    ///
    /// # Panics
    /// This function will panic if something went wrong while setting up the
    /// wallet and there is no address stored at index 0.
    pub fn default_shielded_account(&self) -> Address {
        self.shielded_account(0)
            .expect("there to be an address at index 0")
    }

    /// Returns the default public account address for this wallet
    ///
    /// # Panics
    /// This function will panic if something went wrong while setting up the
    /// wallet and there is no address stored at index 0.
    pub fn default_public_address(&self) -> Address {
        self.public_address(0)
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

    /// get cache database path
    pub(crate) fn cache_path(&self) -> Result<PathBuf, Error> {
        let cache_dir = {
            if let Some(file) = &self.file {
                file.path().cache_dir()
            } else {
                return Err(Error::WalletFileMissing);
            }
        };

        Ok(cache_dir)
    }

    /// Returns the shielded key for a given index.
    ///
    /// # Errors
    /// This will error if the wallet doesn't have a profile stored for the
    /// given index.
    pub fn shielded_key(&self, index: u8) -> Result<&PhoenixPublicKey, Error> {
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

    /// Returns the public account key for a given index.
    ///
    /// # Errors
    /// This will error if the wallet doesn't have a profile stored for the
    /// given index.
    pub fn public_key(&self, index: u8) -> Result<&BlsPublicKey, Error> {
        let index = usize::from(index);
        if index >= self.profiles.len() {
            return Err(Error::Unauthorized);
        }

        Ok(&self.profiles()[index].public_addr)
    }

    /// Returns the public account address for a given index.
    ///
    /// # Errors
    /// This will error if the wallet doesn't have a profile stored for the
    /// given index.
    pub fn public_address(&self, index: u8) -> Result<Address, Error> {
        let addr = *self.public_key(index)?;
        Ok(addr.into())
    }

    /// Returns the shielded account address for a given index.
    ///
    /// # Errors
    /// This method will error if there is no profile stored at the given index.
    pub fn shielded_account(&self, index: u8) -> Result<Address, Error> {
        let addr = *self.shielded_key(index)?;
        Ok(addr.into())
    }

    /// Obtains stake information for a given address.
    ///
    /// # Errors
    /// This method will error if the wallet is not connected to the network or
    /// if there is no profile stored for the given `profile_idx`.
    pub async fn stake_info(
        &self,
        profile_idx: u8,
    ) -> Result<Option<StakeData>, Error> {
        self.state()?
            .fetch_stake(self.public_key(profile_idx)?)
            .await
    }

    /// Returns BLS key-pair for provisioner nodes
    ///
    /// # Errors
    /// This method will error if the given index doesn't exist or if the
    /// internally stored keys are corrupted.
    pub fn provisioner_keys(
        &self,
        index: u8,
    ) -> Result<(BlsPublicKey, BlsSecretKey), Error> {
        let pk = *self.public_key(index)?;
        let sk = self.derive_bls_sk(index);

        // make sure our internal addresses are not corrupted
        if pk != BlsPublicKey::from(&sk) {
            return Err(Error::Unauthorized);
        }

        Ok((pk, sk))
    }

    /// Exports BLS key-pair for provisioners in node-compatible format
    ///
    /// # Errors
    /// This method will error if the provided `dir` is not valid of if the
    /// `profile_idx` doesn't exist in the wallet.
    pub fn export_provisioner_keys(
        &self,
        profile_idx: u8,
        dir: &Path,
        filename: Option<String>,
        pwd: &str,
    ) -> Result<(PathBuf, PathBuf), Error> {
        // we're expecting a directory here
        if !dir.is_dir() {
            return Err(Error::NotDirectory);
        }

        let (pk, sk) = self.provisioner_keys(profile_idx)?;
        let path = PathBuf::from(dir);
        let filename = filename.unwrap_or(profile_idx.to_string());

        Ok(node_data::bls::save_consensus_keys(
            &path, &filename, &pk, &sk, pwd,
        )?)
    }

    /// Return the index of the address passed, returns an error if the address
    /// is not in the wallet profiles.
    ///
    /// # Errors
    /// This method will error if the address is not among the internally stored
    /// addresses.
    pub fn find_index(&self, addr: &Address) -> Result<u8, Error> {
        // check if the key is stored in our profiles, return its index if
        // found
        for (index, profile) in self.profiles().iter().enumerate() {
            if match addr {
                Address::Shielded(addr) => addr == &profile.shielded_addr,
                Address::Public(addr) => addr == &profile.public_addr,
            } {
                // we know that `index < MAX_PROFILES <= u8::MAX`, so
                // casting to u8 is safe here
                #[allow(clippy::cast_possible_truncation)]
                return Ok(index as u8);
            }
        }

        // return an error otherwise
        Err(Error::Unauthorized)
    }

    /// Check if the address is stored in our profiles, return the address if
    /// found
    ///
    /// # Errors
    /// This method will error if the address is not among the internally stored
    /// addresses.
    pub fn claim(&self, addr: Address) -> Result<Address, Error> {
        self.find_index(&addr)?;
        Ok(addr)
    }

    /// Generate a contract id given bytes and nonce
    ///
    /// # Errors
    /// This method will error if the hash maps to an invalid contract-id, this
    /// would mean there is a bug in the `blake2b_simd` hasher.
    pub fn get_contract_id(
        &self,
        profile_idx: u8,
        bytes: &[u8],
        nonce: u64,
    ) -> Result<[u8; CONTRACT_ID_BYTES], Error> {
        let owner = self.public_key(profile_idx)?.to_bytes();

        let mut hasher = blake2b_simd::Params::new()
            .hash_length(CONTRACT_ID_BYTES)
            .to_state();
        hasher.update(bytes);
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
    ///
    /// # Errors
    /// This method will error if the wallet-file cannot be obtained.
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
    ///
    /// # Errors
    /// This method will error if the wallet cannot connect to the network.
    pub async fn is_synced(&self) -> Result<bool, Error> {
        let state = self.state()?;
        let db_pos = state.cache().last_pos()?.unwrap_or(0);
        let num_notes = state.fetch_num_notes().await?;

        // we only subtract if number of notes is higher
        // than 1 to avoid overflow
        let network_last_pos = if num_notes > 1 { num_notes - 1 } else { 0 };

        Ok(network_last_pos == db_pos)
    }

    /// Erase the cache directory
    ///
    /// # Errors
    /// This method will error if the wallet cannot connect to the network.
    pub fn delete_cache(&mut self) -> Result<(), Error> {
        let path = self.cache_path()?;

        std::fs::remove_dir_all(path).map_err(Error::IO)
    }

    /// Close the wallet and zeroize the seed
    pub fn close(&mut self) {
        self.store.inner_mut().zeroize();

        // close the state if exists
        if let Some(x) = &mut self.state {
            x.close();
        }
    }

    /// Get gas prices from the mempool
    ///
    /// # Errors
    /// This method will error if the wallet cannot connect to the network or if
    /// the network response is not valid json.
    pub async fn get_mempool_gas_prices(
        &self,
    ) -> Result<MempoolGasPrices, Error> {
        let client = self.state()?.client();

        let response = client
            .call("blocks", None, "gas-price", &[] as &[u8])
            .await?;

        let gas_prices: MempoolGasPrices = serde_json::from_slice(&response)?;

        Ok(gas_prices)
    }

    /// Get the amount of stake rewards the user has
    ///
    /// # Errors
    /// This method will error if the wallet cannot connect to the network or if
    /// there is no stake recorded for the given sender.
    pub async fn get_stake_reward(
        &self,
        sender_index: u8,
    ) -> Result<Dusk, Error> {
        let available_reward = self
            .stake_info(sender_index)
            .await?
            .ok_or(Error::NotStaked)?
            .reward;

        Ok(Dusk::from(available_reward))
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

#[cfg(test)]
mod tests {
    use aes_gcm::{AeadCore, Aes256Gcm};
    use rand::rngs::OsRng;
    use rand::RngCore;
    use tempfile::tempdir;

    use crate::{IV_SIZE, SALT_SIZE};

    use super::*;

    const TEST_ADDR: &str = "2w7fRQW23Jn9Bgm1GQW9eC2bD9U883dAwqP7HAr2F8g1syzPQaPYrxSyyVZ81yDS5C1rv9L8KjdPBsvYawSx3QCW";

    #[derive(Debug, Clone)]
    struct WalletFile {
        path: WalletPath,
        key: Vec<u8>,
        salt: [u8; SALT_SIZE],
        iv: [u8; IV_SIZE],
    }

    impl SecureWalletFile for WalletFile {
        fn path(&self) -> &WalletPath {
            &self.path
        }

        fn aes_key(&self) -> &[u8] {
            &self.key
        }

        fn salt(&self) -> Option<&[u8; SALT_SIZE]> {
            Some(&self.salt)
        }

        fn iv(&self) -> Option<&[u8; IV_SIZE]> {
            Some(&self.iv)
        }
    }

    #[test]
    fn wallet_basics() -> Result<(), Box<dyn std::error::Error>> {
        // create a wallet from a mnemonic phrase
        let mut wallet: Wallet<WalletFile> = Wallet::new("uphold stove tennis fire menu three quick apple close guilt poem garlic volcano giggle comic")?;

        // check address generation
        let default_addr = wallet.default_shielded_account();
        let other_addr_idx = wallet.add_profile();
        let other_addr =
            Address::Shielded(*wallet.shielded_key(other_addr_idx)?);

        assert!(format!("{default_addr}").eq(TEST_ADDR));
        assert_ne!(default_addr, other_addr);
        assert_eq!(wallet.profiles.len(), 2);

        // create another wallet with different mnemonic
        let wallet: Wallet<WalletFile> = Wallet::new("demise monitor elegant cradle squeeze cheap parrot venture stereo humor scout denial action receive flat")?;

        // check addresses are different
        let addr = wallet.default_shielded_account();
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
        let key = blake3::hash("mypassword".as_bytes()).as_bytes().to_vec();

        // create and save
        let mut wallet: Wallet<WalletFile> = Wallet::new("uphold stove tennis fire menu three quick apple close guilt poem garlic volcano giggle comic")?;
        let salt = gen_salt();
        let iv = gen_iv();
        let file = WalletFile {
            path,
            key,
            salt,
            iv,
        };
        wallet.save_to(file.clone())?;

        // load from file and check
        let loaded_wallet = Wallet::from_file(file)?;

        let original_addr = wallet.default_shielded_account();
        let loaded_addr = loaded_wallet.default_shielded_account();
        assert!(original_addr.eq(&loaded_addr));

        Ok(())
    }

    fn gen_salt() -> [u8; SALT_SIZE] {
        let mut salt = [0; SALT_SIZE];
        let mut rng = OsRng;
        rng.fill_bytes(&mut salt);
        salt
    }

    fn gen_iv() -> [u8; IV_SIZE] {
        let iv = Aes256Gcm::generate_nonce(OsRng);
        iv.into()
    }
}
