// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::{Error, WalletFile};
use dusk_bytes::Serializable;
use dusk_core::abi::ContractId;
use dusk_core::signatures::bls::Signature;
use rusk_wallet::Wallet;
use sha3::{Digest, Sha3_256};
use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Uploads the data driver bytecode for a specified contract
/// Upload message must contain a signature of a bytecode hash.
/// In addition, only contract's owner is eligible for driver upload.
pub async fn driver_upload(
    driver_bytecode_path: impl AsRef<Path>,
    contract_id: &ContractId,
    wallet: &mut Wallet<WalletFile>,
    wallet_index: u8,
) -> Result<(), Error> {
    // Read the driver bytecode
    let mut driver_bytecode_file = File::open(driver_bytecode_path)?;
    let mut driver_bytecode = Vec::new();
    driver_bytecode_file.read_to_end(&mut driver_bytecode)?;

    // Hash the driver bytecode
    let mut hasher = Sha3_256::new();
    hasher.update(&driver_bytecode);
    let hash = hasher.finalize();

    // Sign the hash
    let signature: Signature = wallet.sign(wallet_index, &hash);

    // Convert signature
    let mut signature_vec = Vec::new();
    signature_vec.extend_from_slice(&signature.to_bytes());

    // Call the upload method via http
    let http_client = wallet.state()?.client();
    let _ = http_client
        .upload_driver(&driver_bytecode, contract_id, signature_vec)
        .await?;

    Ok(())
}
