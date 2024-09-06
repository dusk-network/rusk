// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use rusk::{Error, Result};
use rusk_wallet::{SecureWalletFile, Wallet, WalletPath};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct WalletFile {
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

#[allow(dead_code)]
pub fn test_wallet() -> Result<Wallet<WalletFile>, Error> {
    // the file 'test_wallet.dat' needs to be stored in the current dir
    let wallet_path = WalletPath::from(
        std::env::current_dir()?
            .as_path()
            .join("tests")
            .join("common")
            .join("test_wallet.dat"),
    );
    let wallet_file = WalletFile {
        path: wallet_path,
        pwd: blake3::hash(b"mypassword").as_bytes().to_vec(),
    };

    // Load wallet from the wallet-file, which is a wallet with the seed of
    // an array of `0u8`. Check 'rusk-wallet/tests/generate_test_wallet.rs'
    // on how to generate such a wallet-file.
    println!("the path is: {}", wallet_file.path);
    let wallet =
        Wallet::from_file(wallet_file).map_err(|_| Error::RestoreFailed)?;

    Ok(wallet)
}
