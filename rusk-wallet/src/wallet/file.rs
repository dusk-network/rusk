// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::dat::DatFileVersion;
use crate::{Error, SecureWalletFile};

use super::file_service::WalletFilePath;

/// Wallet file structure that contains the path of the wallet file, the hashed
/// password, and the file version
#[derive(Debug, Clone)]
pub struct WalletFile {
    path: WalletPath,
    pwd: Vec<u8>,
    file_version: DatFileVersion,
}

impl SecureWalletFile for WalletFile {
    type PathBufWrapper = WalletPath;

    fn path(&self) -> &WalletPath {
        &self.path
    }

    fn path_mut(&mut self) -> &mut WalletPath {
        &mut self.path
    }

    fn pwd(&self) -> &[u8] {
        &self.pwd
    }

    fn version(&self) -> DatFileVersion {
        self.file_version
    }
}

impl WalletFile {
    /// Create a new wallet file
    pub fn new(
        path: WalletPath,
        pwd: Vec<u8>,
        file_version: DatFileVersion,
    ) -> Self {
        Self {
            path,
            pwd,
            file_version,
        }
    }
}

/// Wrapper around `PathBuf` for wallet paths
#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub struct WalletPath {
    /// Path to the wallet file
    wallet: PathBuf,
    /// Directory of the profile
    profile_dir: PathBuf,
    /// Name of the network
    network: Option<String>,
}

impl WalletFilePath for WalletPath {
    fn wallet_path(&self) -> &PathBuf {
        &self.wallet
    }

    fn wallet_path_mut(&mut self) -> &mut PathBuf {
        &mut self.wallet
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

impl WalletPath {
    /// Create wallet path from the path of "wallet.dat" file. The wallet.dat
    /// file should be located in the profile folder, this function also
    /// generates the profile folder from the passed argument
    pub fn new(wallet_file_path: &Path) -> Result<Self, Error> {
        let wallet = wallet_file_path.to_path_buf();
        // The wallet should be in the profile folder
        let mut profile_dir = wallet.clone();

        let is_valid_dir = profile_dir.pop();

        if !is_valid_dir {
            return Err(Error::InvalidWalletFilePath);
        }

        Ok(Self {
            wallet,
            profile_dir,
            network: None,
        })
    }
}

impl TryFrom<PathBuf> for WalletPath {
    type Error = Error;

    fn try_from(p: PathBuf) -> Result<Self, Self::Error> {
        let p = p.to_path_buf();

        let is_valid =
            p.try_exists().map_err(|_| Error::InvalidWalletFilePath)?
                && p.is_file();

        if !is_valid {
            return Err(Error::InvalidWalletFilePath);
        }

        Self::new(&p)
    }
}

impl TryFrom<&Path> for WalletPath {
    type Error = Error;

    fn try_from(p: &Path) -> Result<Self, Self::Error> {
        let p = p.to_path_buf();

        Self::try_from(p)
    }
}

impl FromStr for WalletPath {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let p = Path::new(s);

        Self::try_from(p)
    }
}

impl Display for WalletPath {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_wallet_path_creation() -> Result<(), Error> {
        let dir = tempdir()?;
        let wallet_file = dir.path().join("wallet.dat");
        let file = File::create(&wallet_file)?;

        let wallet_path = WalletPath::new(&wallet_file)?;

        assert_eq!(wallet_path.wallet_path(), &wallet_file, "wallet path is not correct for WalletPath created by WalletPath::new method");
        assert_eq!(wallet_path.profile_dir(), dir.path(), "profile dir is not correct for WalletPath created by WalletPath::new method");
        assert_eq!(wallet_path.network(), None, "network is not correct for WalletPath created by WalletPath::new method");

        // try_from(PathBuf)
        let wallet_path = WalletPath::try_from(wallet_file.clone())?;

        assert_eq!(wallet_path.wallet_path(), &wallet_file, "wallet path is not correct for WalletPath created by WalletPath::try_from(PathBuf) method");
        assert_eq!(wallet_path.profile_dir(), dir.path(), "profile dir is not correct for WalletPath created by WalletPath::try_from(PathBuf) method");
        assert_eq!(wallet_path.network(), None, "network is not correct for WalletPath created by WalletPath::try_from(PathBuf) method");

        // try_from(&Path)
        let wallet_path = WalletPath::try_from(wallet_file.as_path())?;

        assert_eq!(wallet_path.wallet_path(), &wallet_file, "wallet path is not correct for WalletPath created by WalletPath::try_from(&Path) method");
        assert_eq!(wallet_path.profile_dir(), dir.path(), "profile dir is not correct for WalletPath created by WalletPath::try_from(&Path) method");
        assert_eq!(wallet_path.network(), None, "network is not correct for WalletPath created by WalletPath::try_from(&Path) method");

        // from_str
        let wallet_path = WalletPath::from_str(wallet_file.to_str().unwrap())?;

        assert_eq!(wallet_path.wallet_path(), &wallet_file, "wallet path is not correct for WalletPath created by WalletPath::from_str method");
        assert_eq!(wallet_path.profile_dir(), dir.path(), "profile dir is not correct for WalletPath created by WalletPath::from_str method");
        assert_eq!(wallet_path.network(), None, "network is not correct for WalletPath created by WalletPath::from_str method");

        // the path is not a file
        let wallet_path = WalletPath::try_from(dir.path());

        assert!(
            wallet_path.is_err(),
            "WalletPath::try_from should return an error when the path is not a file"
        );

        // the path does not exist
        let wallet_path = WalletPath::from_str("invalid_path");

        assert!(
            wallet_path.is_err(),
            "WalletPath::try_from should return an error when the path does not exist"
        );

        drop(file);
        dir.close()?;

        Ok(())
    }

    #[test]
    fn test_wallet_file_creation() -> Result<(), Error> {
        let dir = tempdir()?;
        let wallet_file = dir.path().join("wallet.dat");
        let file = File::create(&wallet_file)?;

        let path = WalletPath::new(&wallet_file)?;
        let pwd = vec![1, 2, 3, 4];
        let file_version =
            DatFileVersion::RuskBinaryFileFormat((1, 0, 0, 0, false));

        let wallet_file =
            WalletFile::new(path.clone(), pwd.clone(), file_version);

        assert_eq!(
            wallet_file.path(),
            &path,
            "path is not correct for WalletFile"
        );
        assert_eq!(
            wallet_file.pwd(),
            &pwd,
            "pwd is not correct for WalletFile"
        );
        assert_eq!(
            wallet_file.version(),
            file_version,
            "file_version is not correct for WalletFile"
        );

        drop(file);
        dir.close()?;

        Ok(())
    }
}
