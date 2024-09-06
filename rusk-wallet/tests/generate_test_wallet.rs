// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use wallet_core::keys::RNG_SEED;

// Create a wallet for testing where the seed is an array of `0u8`.
//
// Since there is no functionality to override the seed or address-count in
// a wallet (and there also shouldn't be one), we modified the `save` method
// (called by `save_to`) of the wallet so that the seed is overridden with
// `[0u8; RNG_SEED]` and the address count with `100`.
//
// For the asserts to work the `store` field must be pub, as well as the
// `LocalStore` struct
//
// Create the wallet-file by doing the above adjustments, uncomment the
// `#[test]` line below and run
// `cargo test --release --test generate_test_wallet`.
// The generated 'test_wallet.dat' file can then be moved to where it is needed.
//
// #[test]
fn create_test_wallet() -> Result<(), rusk_wallet::Error> {
    use rusk_wallet::{SecureWalletFile, Wallet, WalletPath};

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

    // create the wallet file path
    let wallet_path = WalletPath::from(
        std::env::current_dir()?.as_path().join("test_wallet.dat"),
    );
    let wallet_file = WalletFile {
        path: wallet_path,
        pwd: blake3::hash(b"mypassword").as_bytes().to_vec(),
    };

    // create a test wallet with an arbitrary passphrase (the seed will be
    // overridden)
    const PHRASE: &str = "park remain person kitchen mule spell knee armed position rail grid ankle";

    let mut wallet = Wallet::new(PHRASE)?;

    // since there is no functionality to override the seed or address-count in
    // a wallet (and there also shouldn't be one), we modified the `save` method
    // (called by `save_to`) of the wallet so that the seed is overridden with
    // `[0u8; RNG_SEED]` and the address count with `100`.
    wallet.save_to(wallet_file.clone())?;

    // load wallet from file and check seed and address count
    // for these assert to work the `store` field must be pub, as well as the
    // `LocalStore` struct
    const SEED: [u8; RNG_SEED] = [0u8; RNG_SEED];
    const ADDR_COUNT: u8 = 100;

    let loaded_wallet = Wallet::from_file(wallet_file)?;
    assert_eq!(*loaded_wallet.store.get_seed(), SEED);
    assert_eq!(loaded_wallet.addresses().len(), ADDR_COUNT as usize);
    Ok(())
}
