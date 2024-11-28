// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fmt::Debug;
use std::hash::Hash;
use std::path::PathBuf;

use wallet_core::Seed;

use crate::crypto::decrypt;
use crate::dat::DatFileVersion;
use crate::Error;

/// Provides access to a secure wallet file
pub trait SecureWalletFile: Debug + Send + Sync + Clone {
    /// The type of the path buffer wrapper
    type PathBufWrapper: WalletFilePath + Hash + Eq + PartialEq + Debug + Clone;

    // Methods to implement ===================================================

    /// Returns the path
    fn path(&self) -> &Self::PathBufWrapper;
    /// Return the mutable path
    fn path_mut(&mut self) -> &mut Self::PathBufWrapper;
    /// Returns the hashed password
    fn pwd(&self) -> &[u8];
    /// Returns the file version
    fn version(&self) -> DatFileVersion;

    // Automatically implemented methods =======================================

    /// Returns the path of the wallet file
    fn wallet_path(&self) -> &PathBuf {
        self.path().wallet_path()
    }

    /// Returns the directory of the profile
    fn profile_dir(&self) -> &PathBuf {
        self.path().profile_dir()
    }

    /// Returns the network name for different cache locations
    /// e.g, devnet, testnet, etc.
    fn network(&self) -> Option<&String> {
        self.path().network()
    }

    /// Sets the network name for different cache locations.
    /// e.g, devnet, testnet, etc.
    fn set_network_name(&mut self, network: Option<String>) {
        self.path_mut().set_network_name(network);
    }

    /// Returns the filename of this file
    fn name(&self) -> Option<String> {
        self.path().name()
    }

    /// Returns dir for cache based on network specified
    fn cache_dir(&self) -> PathBuf {
        self.path().cache_dir()
    }

    /// Checks if the file version is older than the latest Rust Binary file
    /// format
    fn is_old(&self) -> bool {
        let version = self.version();
        matches!(
            version,
            DatFileVersion::Legacy | DatFileVersion::OldWalletCli(_)
        )
    }

    /// Get the seed and address from the file
    fn get_seed_and_address(&self) -> Result<(Seed, u8), Error> {
        let file_version = self.version();
        let pwd = self.pwd();
        let wallet_path = self.wallet_path().clone();

        // Make sure the wallet file exists
        if !wallet_path.is_file() {
            return Err(Error::WalletFileMissing);
        }

        // Load the wallet file
        let mut bytes = std::fs::read(wallet_path)?;

        match file_version {
            DatFileVersion::Legacy => {
                if bytes[1] == 0 && bytes[2] == 0 {
                    bytes.drain(..3);
                }

                bytes = decrypt(&bytes, pwd)?;

                let seed = bytes[..]
                    .try_into()
                    .map_err(|_| Error::WalletFileCorrupted)?;

                Ok((seed, 1))
            }
            DatFileVersion::OldWalletCli((major, minor, _, _, _)) => {
                bytes.drain(..5);

                let content = decrypt(&bytes, pwd)?;
                let buff = &content[..];

                let seed =
                    buff.try_into().map_err(|_| Error::WalletFileCorrupted)?;

                match (major, minor) {
                    (1, 0) => Ok((seed, 1)),
                    (2, 0) => Ok((seed, buff[0])),
                    _ => Err(Error::UnknownFileVersion(major, minor)),
                }
            }
            DatFileVersion::RuskBinaryFileFormat(_) => {
                let rest = bytes.get(12..(12 + 96));

                if let Some(rest) = rest {
                    let content = decrypt(rest, pwd)?;

                    if let Some(seed_buf) = content.get(0..65) {
                        let seed = seed_buf[0..64]
                            .try_into()
                            .map_err(|_| Error::WalletFileCorrupted)?;

                        let addr_count = &seed_buf[64..65];

                        Ok((seed, addr_count[0]))
                    } else {
                        Err(Error::WalletFileCorrupted)
                    }
                } else {
                    Err(Error::WalletFileCorrupted)
                }
            }
        }
    }
}

/// Provides access to the wallet file path, profile directory and network name,
/// and implements by default other useful methods
pub trait WalletFilePath {
    // Methods to implement ===================================================

    /// Returns the path of the wallet file
    fn wallet_path(&self) -> &PathBuf;
    /// Returns the mutable path of the wallet file
    fn wallet_path_mut(&mut self) -> &mut PathBuf;
    /// Returns the directory of the profile
    fn profile_dir(&self) -> &PathBuf;
    /// Returns the network name for different cache locations
    /// e.g, devnet, testnet, etc.
    fn network(&self) -> Option<&String>;
    /// Returns the mutable network name
    fn network_mut(&mut self) -> &mut Option<String>;

    // Automatically implemented methods =======================================

    /// Sets the network name for different cache locations.
    /// e.g, devnet, testnet, etc.
    fn set_network_name(&mut self, network: Option<String>) {
        *self.network_mut() = network;
    }

    /// Returns the filename of this path
    fn name(&self) -> Option<String> {
        // extract the name
        let name = self.wallet_path().file_stem()?.to_str()?;
        Some(String::from(name))
    }

    /// Returns dir for cache based on network specified
    fn cache_dir(&self) -> PathBuf {
        let mut cache = self.profile_dir().clone();

        if let Some(network) = self.network() {
            cache.push(format!("cache_{network}"));
        } else {
            cache.push("cache");
        }

        cache
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(PartialEq, Eq, Hash, Debug, Clone)]
    struct MockWalletFilePath {
        pub wallet_path: PathBuf,
        pub profile_dir: PathBuf,
        pub network: Option<String>,
    }

    impl WalletFilePath for MockWalletFilePath {
        fn wallet_path(&self) -> &PathBuf {
            &self.wallet_path
        }

        fn wallet_path_mut(&mut self) -> &mut PathBuf {
            &mut self.wallet_path
        }

        fn profile_dir(&self) -> &PathBuf {
            &self.profile_dir
        }

        fn network(&self) -> Option<&String> {
            self.network.as_ref()
        }

        fn network_mut(&mut self) -> &mut Option<String> {
            &mut self.network
        }
    }

    #[derive(Debug, Clone)]
    struct MockSecureWalletFile {
        pub path: MockWalletFilePath,
        pub pwd: Vec<u8>,
        pub version: DatFileVersion,
    }

    impl SecureWalletFile for MockSecureWalletFile {
        type PathBufWrapper = MockWalletFilePath;

        fn path(&self) -> &Self::PathBufWrapper {
            &self.path
        }

        fn path_mut(&mut self) -> &mut Self::PathBufWrapper {
            &mut self.path
        }

        fn pwd(&self) -> &[u8] {
            &self.pwd
        }

        fn version(&self) -> DatFileVersion {
            self.version.clone()
        }
    }

    #[test]
    fn test_secure_wallet_file_trait_methods() -> Result<(), Error> {
        let file_path = PathBuf::from("wallet.dat");
        let profile_dir = PathBuf::from("profile");
        let network = Some("devnet".to_string());

        let pwd = vec![1, 2, 3, 4];
        let version = DatFileVersion::RuskBinaryFileFormat((1, 0, 0, 0, false));

        let wallet_path = MockWalletFilePath {
            wallet_path: file_path.clone(),
            profile_dir: profile_dir.clone(),
            network: network.clone(),
        };

        let mut wallet_file = MockSecureWalletFile {
            path: wallet_path.clone(),
            pwd: pwd.clone(),
            version: version.clone(),
        };

        assert_eq!(
            wallet_file.wallet_path(),
            wallet_path.wallet_path(),
            "wallet path is not correct for SecureWalletFile"
        );
        assert_eq!(
            wallet_file.profile_dir(),
            &profile_dir,
            "profile dir is not correct for SecureWalletFile"
        );
        assert_eq!(
            wallet_file.network(),
            network.as_ref(),
            "network is not correct for SecureWalletFile"
        );

        let network = Some("testnet".to_string());

        wallet_file.set_network_name(network.clone());

        assert_eq!(
            wallet_file.network(),
            network.as_ref(),
            "network is not correct for SecureWalletFile after set_network_name"
        );

        assert_eq!(
            wallet_file.name(),
            Some("wallet".to_string()),
            "name is not correct for SecureWalletFile"
        );

        assert_eq!(
            wallet_file.cache_dir(),
            PathBuf::from("profile/cache_testnet"),
            "cache_dir is not correct for SecureWalletFile"
        );

        assert!(
            !wallet_file.is_old(),
            "is_old is not correct for SecureWalletFile"
        );

        let old_file = MockSecureWalletFile {
            path: wallet_path.clone(),
            pwd: pwd.clone(),
            version: DatFileVersion::Legacy,
        };

        assert!(
            old_file.is_old(),
            "is_old is not correct for SecureWalletFile with old file"
        );

        let another_old_file = MockSecureWalletFile {
            path: wallet_path.clone(),
            pwd: pwd.clone(),
            version: DatFileVersion::OldWalletCli((1, 0, 0, 0, false)),
        };

        assert!(
            another_old_file.is_old(),
            "is_old is not correct for SecureWalletFile with another old file"
        );

        // TODO: test get_seed_and_address

        Ok(())
    }
}
