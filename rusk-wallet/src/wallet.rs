// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod address;
mod file;
pub mod gas;

pub use address::Address;
use dusk_plonk::prelude::BlsScalar;
pub use file::{SecureWalletFile, WalletPath};

use bip39::{Language, Mnemonic, Seed};
use dusk_bytes::{DeserializableSlice, Serializable};
use ff::Field;
use flume::Receiver;
use phoenix_core::transaction::ModuleId;
use phoenix_core::Note;
use rkyv::ser::serializers::AllocSerializer;
use serde::Serialize;
use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};

use dusk_bls12_381_sign::{PublicKey, SecretKey};
use dusk_wallet_core::{
    BalanceInfo, StakeInfo, StateClient, Store, Transaction,
    Wallet as WalletCore, MAX_CALL_SIZE,
};
use rand::prelude::StdRng;
use rand::SeedableRng;

use dusk_pki::{PublicSpendKey, SecretSpendKey};

use crate::cache::NoteData;
use crate::clients::{Prover, StateStore};
use crate::crypto::encrypt;
use crate::currency::Dusk;
use crate::dat::{
    self, version_bytes, DatFileVersion, FILE_TYPE, LATEST_VERSION, MAGIC,
    RESERVED,
};
use crate::store::LocalStore;
use crate::{Error, RuskHttpClient};
use gas::Gas;

use crate::store;

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
    wallet: Option<WalletCore<LocalStore, StateStore, Prover>>,
    addresses: Vec<Address>,
    store: LocalStore,
    file: Option<F>,
    file_version: Option<DatFileVersion>,
    status: fn(status: &str),
    /// Recieve the status/errors of the sync procss
    pub sync_rx: Option<Receiver<String>>,
}

impl<F: SecureWalletFile + Debug> Wallet<F> {
    /// Returns the file used for the wallet
    pub fn file(&self) -> &Option<F> {
        &self.file
    }

    /// Returns spending key pair for a given address
    pub fn spending_keys(
        &self,
        addr: &Address,
    ) -> Result<(PublicSpendKey, SecretSpendKey), Error> {
        // make sure we own the address
        if !addr.is_owned() {
            return Err(Error::Unauthorized);
        }

        let index = addr.index()? as u64;

        // retrieve keys
        let ssk = self.store.retrieve_ssk(index)?;
        let psk: PublicSpendKey = ssk.public_spend_key();

        Ok((psk, ssk))
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
            let mut bytes = seed.as_bytes();

            // Generate a Store Seed type from the mnemonic Seed bytes
            let seed = store::Seed::from_reader(&mut bytes)?;

            let store = LocalStore::new(seed);

            // Generate the default address
            let ssk = store
                .retrieve_ssk(0)
                .expect("wallet seed should be available");

            let address = Address::new(0, ssk.public_spend_key());

            // return new wallet instance
            Ok(Wallet {
                wallet: None,
                addresses: vec![address],
                store,
                file: None,
                file_version: None,
                status: |_| {},
                sync_rx: None,
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

        let store = LocalStore::new(seed);

        // return early if its legacy
        if let DatFileVersion::Legacy = file_version {
            let ssk = store
                .retrieve_ssk(0)
                .expect("wallet seed should be available");

            let address = Address::new(0, ssk.public_spend_key());

            // return the store
            return Ok(Self {
                wallet: None,
                addresses: vec![address],
                store,
                file: Some(file),
                file_version: Some(DatFileVersion::Legacy),
                status: |_| {},
                sync_rx: None,
            });
        }

        let addresses: Vec<_> = (0..address_count)
            .map(|i| {
                let ssk = store
                    .retrieve_ssk(i as u64)
                    .expect("wallet seed should be available");

                Address::new(i, ssk.public_spend_key())
            })
            .collect();

        // create and return
        Ok(Self {
            wallet: None,
            addresses,
            store,
            file: Some(file),
            file_version: Some(file_version),
            status: |_| {},
            sync_rx: None,
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
                let seed = self.store.get_seed()?;
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
        let http_state = RuskHttpClient::new(rusk_addr.into());
        let http_prover = RuskHttpClient::new(prov_addr.into());

        let state_status = http_state.check_connection().await;
        let prover_status = http_prover.check_connection().await;

        match (&state_status, prover_status) {
            (Err(e),_)=> println!("Connection to Rusk Failed, some operations won't be available: {e}"),
            (_,Err(e))=> println!("Connection to Prover Failed, some operations won't be available: {e}"),
            _=> {},
        }

        // create a prover client
        let mut prover = Prover::new(http_state.clone(), http_prover.clone());
        prover.set_status_callback(status);

        let cache_dir = {
            if let Some(file) = &self.file {
                file.path().cache_dir()
            } else {
                return Err(Error::WalletFileMissing);
            }
        };

        // create a state client
        let state = StateStore::new(
            http_state,
            &cache_dir,
            self.store.clone(),
            status,
        )?;

        // create wallet instance
        self.wallet = Some(WalletCore::new(self.store.clone(), state, prover));

        // set our own status callback
        self.status = status;

        Ok(())
    }

    /// Sync wallet state
    pub async fn sync(&self) -> Result<(), Error> {
        self.connected_wallet().await?.state().sync().await
    }

    /// Helper function to register for async-sync outside of connect
    pub async fn register_sync(&mut self) -> Result<(), Error> {
        match self.wallet.as_ref() {
            Some(w) => {
                let (sync_tx, sync_rx) = flume::unbounded::<String>();
                w.state().register_sync(sync_tx).await?;
                self.sync_rx = Some(sync_rx);
                Ok(())
            }
            None => Err(Error::Offline),
        }
    }

    /// Checks if the wallet has an active connection to the network
    pub async fn is_online(&self) -> bool {
        match self.wallet.as_ref() {
            Some(w) => w.state().check_connection().await.is_ok(),
            None => false,
        }
    }

    pub(crate) async fn connected_wallet(
        &self,
    ) -> Result<&WalletCore<LocalStore, StateStore, Prover>, Error> {
        match self.wallet.as_ref() {
            Some(w) => {
                w.state().check_connection().await?;
                Ok(w)
            }
            None => Err(Error::Offline),
        }
    }

    /// Fetches the notes from the state.
    pub async fn get_all_notes(
        &self,
        addr: &Address,
    ) -> Result<Vec<DecodedNote>, Error> {
        if !addr.is_owned() {
            return Err(Error::Unauthorized);
        }

        let wallet = self.connected_wallet().await?;
        let ssk_index = addr.index()? as u64;
        let ssk = self.store.retrieve_ssk(ssk_index).unwrap();
        let vk = ssk.view_key();
        let psk = vk.public_spend_key();

        let live_notes = wallet.state().fetch_notes(&vk).unwrap();
        let spent_notes = wallet.state().cache().spent_notes(&psk)?;

        let live_notes = live_notes
            .into_iter()
            .map(|(note, height)| (None, note, height));
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
    pub async fn get_balance(
        &self,
        addr: &Address,
    ) -> Result<BalanceInfo, Error> {
        // make sure we own this address
        if !addr.is_owned() {
            return Err(Error::Unauthorized);
        }

        // get balance
        if let Some(wallet) = &self.wallet {
            let index = addr.index()? as u64;
            Ok(wallet.get_balance(index)?)
        } else {
            Err(Error::Offline)
        }
    }

    /// Creates a new public address.
    /// The addresses generated are deterministic across sessions.
    pub fn new_address(&mut self) -> &Address {
        let len = self.addresses.len();
        let ssk = self
            .store
            .retrieve_ssk(len as u64)
            .expect("wallet seed should be available");
        let addr = Address::new(len as u8, ssk.public_spend_key());

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

    /// Executes a generic contract call
    pub async fn execute<C>(
        &self,
        sender: &Address,
        contract_id: ModuleId,
        call_name: String,
        call_data: C,
        gas: Gas,
    ) -> Result<Transaction, Error>
    where
        C: rkyv::Serialize<AllocSerializer<MAX_CALL_SIZE>>,
    {
        let wallet = self.connected_wallet().await?;
        // make sure we own the sender address
        if !sender.is_owned() {
            return Err(Error::Unauthorized);
        }

        // check gas limits
        if !gas.is_enough() {
            return Err(Error::NotEnoughGas);
        }

        let mut rng = StdRng::from_entropy();
        let sender_index =
            sender.index().expect("owned address should have an index");

        // transfer
        let tx = wallet.execute(
            &mut rng,
            contract_id.into(),
            call_name,
            call_data,
            sender_index as u64,
            sender.psk(),
            gas.limit,
            gas.price,
        )?;
        Ok(tx)
    }

    /// Transfers funds between addresses
    pub async fn transfer(
        &self,
        sender: &Address,
        rcvr: &Address,
        amt: Dusk,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        let wallet = self.connected_wallet().await?;
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

        let mut rng = StdRng::from_entropy();
        let ref_id = BlsScalar::random(&mut rng);
        let sender_index =
            sender.index().expect("owned address should have an index");

        // transfer
        let tx = wallet.transfer(
            &mut rng,
            sender_index as u64,
            sender.psk(),
            rcvr.psk(),
            *amt,
            gas.limit,
            gas.price,
            ref_id,
        )?;
        Ok(tx)
    }

    /// Stakes Dusk
    pub async fn stake(
        &self,
        addr: &Address,
        amt: Dusk,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        let wallet = self.connected_wallet().await?;
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

        let mut rng = StdRng::from_entropy();
        let sender_index = addr.index()?;

        // stake
        let tx = wallet.stake(
            &mut rng,
            sender_index as u64,
            sender_index as u64,
            addr.psk(),
            *amt,
            gas.limit,
            gas.price,
        )?;
        Ok(tx)
    }

    /// Obtains stake information for a given address
    pub async fn stake_info(&self, addr: &Address) -> Result<StakeInfo, Error> {
        let wallet = self.connected_wallet().await?;
        // make sure we own the staking address
        if !addr.is_owned() {
            return Err(Error::Unauthorized);
        }
        let index = addr.index()? as u64;
        wallet.get_stake(index).map_err(Error::from)
    }

    /// Unstakes Dusk
    pub async fn unstake(
        &self,
        addr: &Address,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        let wallet = self.connected_wallet().await?;
        // make sure we own the staking address
        if !addr.is_owned() {
            return Err(Error::Unauthorized);
        }

        let mut rng = StdRng::from_entropy();
        let index = addr.index()? as u64;

        let tx = wallet.unstake(
            &mut rng,
            index,
            index,
            addr.psk(),
            gas.limit,
            gas.price,
        )?;
        Ok(tx)
    }

    /// Withdraw accumulated staking reward for a given address
    pub async fn withdraw_reward(
        &self,
        addr: &Address,
        gas: Gas,
    ) -> Result<Transaction, Error> {
        let wallet = self.connected_wallet().await?;
        // make sure we own the staking address
        if !addr.is_owned() {
            return Err(Error::Unauthorized);
        }

        let mut rng = StdRng::from_entropy();
        let index = addr.index()? as u64;

        let tx = wallet.withdraw(
            &mut rng,
            index,
            index,
            addr.psk(),
            gas.limit,
            gas.price,
        )?;
        Ok(tx)
    }

    /// Returns bls key pair for provisioner nodes
    pub fn provisioner_keys(
        &self,
        addr: &Address,
    ) -> Result<(PublicKey, SecretKey), Error> {
        // make sure we own the staking address
        if !addr.is_owned() {
            return Err(Error::Unauthorized);
        }

        let index = addr.index()? as u64;

        // retrieve keys
        let sk = self.store.retrieve_sk(index)?;
        let pk: PublicKey = From::from(&sk);

        Ok((pk, sk))
    }

    /// Export bls key pair for provisioners in node-compatible format
    pub fn export_keys(
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
            .find(|a| a.psk == addr.psk)
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
