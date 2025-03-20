// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::{IV_SIZE, SALT_SIZE};

/// Provides access to a secure wallet file
pub trait SecureWalletFile {
    /// Returns the path
    fn path(&self) -> &WalletPath;
    /// Returns the hashed password
    fn aes_key(&self) -> &[u8];
    /// Returns the seed used to hash the password
    fn salt(&self) -> Option<&[u8; SALT_SIZE]>;
    /// Returns the IV used to encrypt/decrypt wallet data
    fn iv(&self) -> Option<&[u8; IV_SIZE]>;
}

/// Wrapper around `PathBuf` for wallet paths
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct WalletPath {
    /// Path of the wallet file
    pub wallet: PathBuf,
    /// Directory of the profile
    pub profile_dir: PathBuf,
    /// Name of the network
    pub network: Option<String>,
}

impl WalletPath {
    /// Create wallet path from the path of "wallet.dat" file. The wallet.dat
    /// file should be located in the profile folder, this function also
    /// generates the profile folder from the passed argument
    pub fn new(wallet: &Path) -> Self {
        let wallet = wallet.to_path_buf();
        // The wallet should be in the profile folder
        let mut profile_dir = wallet.clone();

        profile_dir.pop();

        Self {
            wallet,
            profile_dir,
            network: None,
        }
    }

    /// Returns the filename of this path
    pub fn name(&self) -> Option<String> {
        // extract the name
        let name = self.wallet.file_stem()?.to_str()?;
        Some(String::from(name))
    }

    /// Returns current directory for this path
    pub fn dir(&self) -> Option<PathBuf> {
        self.wallet.parent().map(PathBuf::from)
    }

    /// Returns a reference to the `PathBuf` holding the path
    pub fn inner(&self) -> &PathBuf {
        &self.wallet
    }

    /// Sets the network name for different cache locations.
    /// e.g, devnet, testnet, etc.
    pub fn set_network_name(&mut self, network: Option<String>) {
        self.network = network;
    }

    /// Generates dir for cache based on network specified
    pub fn cache_dir(&self) -> PathBuf {
        let mut cache = self.profile_dir.clone();

        if let Some(network) = &self.network {
            cache.push(format!("cache_{network}"));
        } else {
            cache.push("cache");
        }

        cache
    }
}

impl FromStr for WalletPath {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let p = Path::new(s);

        Ok(Self::new(p))
    }
}

impl From<PathBuf> for WalletPath {
    fn from(p: PathBuf) -> Self {
        Self::new(&p)
    }
}

impl From<&Path> for WalletPath {
    fn from(p: &Path) -> Self {
        Self::new(p)
    }
}

impl fmt::Display for WalletPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "wallet path: {}\n\rprofile dir: {}\n\rnetwork: {}",
            self.wallet.display(),
            self.profile_dir.display(),
            self.network.as_ref().unwrap_or(&"default".to_string())
        )
    }
}
