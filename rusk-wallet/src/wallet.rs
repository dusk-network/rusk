// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod address;
mod file;
mod transaction;

pub use address::{Address, Profile};
pub use file::{SecureWalletFile, WalletPath};

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
use serde::Serialize;
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
    self, version_bytes, DatFileVersion, FILE_TYPE, LATEST_VERSION, MAGIC,
    RESERVED,
};
use crate::gas::MempoolGasPrices;
use crate::rues::RuesHttpClient;
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
    pub async fn connect_with_status<S: Into<String>>(
        &mut self,
        rusk_addr: S,
        prov_addr: S,
        status: fn(&str),
    ) -> Result<(), Error> {
        // attempt connection
        let http_state = RuesHttpClient::new(rusk_addr)?;
        let http_prover = RuesHttpClient::new(prov_addr)?;

        let state_status = http_state.check_connection().await;
        let prover_status = http_prover.check_connection().await;

        match (&state_status, prover_status) {
            (Err(e),_)=> println!("Connection to Rusk Failed, some operations won't be available: {e}"),
            (_,Err(e))=> println!("Connection to Prover Failed, some operations won't be available: {e}"),
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
        if let Some(state) = &self.state {
            state.check_connection().await
        } else {
            false
        }
    }

    /// Fetches the notes from the state.
    pub async fn get_all_notes(
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
    pub fn default_shielded_account(&self) -> Address {
        self.shielded_account(0)
            .expect("there to be an address at index 0")
    }

    /// Returns the default public account address for this wallet
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
    pub fn public_address(&self, index: u8) -> Result<Address, Error> {
        let addr = *self.public_key(index)?;
        Ok(addr.into())
    }

    /// Returns the shielded account address for a given index.
    pub fn shielded_account(&self, index: u8) -> Result<Address, Error> {
        let addr = *self.shielded_key(index)?;
        Ok(addr.into())
    }

    /// Obtains stake information for a given address.
    pub async fn stake_info(
        &self,
        profile_idx: u8,
    ) -> Result<Option<StakeData>, Error> {
        self.state()?
            .fetch_stake(self.public_key(profile_idx)?)
            .await
    }

    /// Returns BLS key-pair for provisioner nodes
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
                Address::Shielded(addr) => addr == &profile.shielded_addr,
                Address::Public(addr) => addr == &profile.public_addr,
            } {
                return Ok(index as u8);
            }
        }

        // return an error otherwise
        Err(Error::Unauthorized)
    }

    /// Check if the address is stored in our profiles, return the address if
    /// found
    pub fn claim(&self, addr: Address) -> Result<Address, Error> {
        self.find_index(&addr)?;
        Ok(addr)
    }

    /// Generate a contract id given bytes and nonce
    pub fn get_contract_id(
        &self,
        profile_idx: u8,
        bytes: Vec<u8>,
        nonce: u64,
    ) -> Result<[u8; CONTRACT_ID_BYTES], Error> {
        let owner = self.public_key(profile_idx)?.to_bytes();

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
    pub async fn is_synced(&self) -> Result<bool, Error> {
        let state = self.state()?;
        let db_pos = state.cache().last_pos()?.unwrap_or(0);
        let network_last_pos = state.fetch_num_notes().await? - 1;

        Ok(network_last_pos == db_pos)
    }

    /// Erase the cache directory
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
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine;
    use serde::{Serialize, Serializer};

    pub fn serialize<S: Serializer>(v: &[u8], s: S) -> Result<S::Ok, S::Error> {
        let base64 = BASE64.encode(v);
        String::serialize(&base64, s)
    }
}

#[cfg(test)]
mod tests {

    use tempfile::tempdir;

    use super::*;

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
        let pwd = blake3::hash("mypassword".as_bytes()).as_bytes().to_vec();

        // create and save
        let mut wallet: Wallet<WalletFile> = Wallet::new("uphold stove tennis fire menu three quick apple close guilt poem garlic volcano giggle comic")?;
        let file = WalletFile { path, pwd };
        wallet.save_to(file.clone())?;

        // load from file and check
        let loaded_wallet = Wallet::from_file(file)?;

        let original_addr = wallet.default_shielded_account();
        let loaded_addr = loaded_wallet.default_shielded_account();
        assert!(original_addr.eq(&loaded_addr));

        Ok(())
    }
}
