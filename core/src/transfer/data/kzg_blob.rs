// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Extra data that may be sent with the `data` field of either transaction
//! type.

use alloc::string::ToString;
use c_kzg::{ethereum_kzg_settings, Blob as KzgBlob, Bytes48};

use super::{BlobData, BlobSidecar, Error};

impl BlobData {
    /// Creates a default `c_kzg::KzgSettings` for KZG operations.
    ///
    /// This function initializes and returns a static reference to a
    /// `c_kzg::KzgSettings` instance, which can be used for Ethereum KZG
    /// operations. If no `precompute` value is provided, the default value
    /// of `0` is used, which is recommended for Ethereum's KZG settings.
    ///
    /// **Note:**  
    /// This function uses [`once_cell::race::OnceBox`] for lazy
    /// initialization, ensuring that the settings are created only once and
    /// reused across calls. If called multiple times with different
    /// `precompute` values, only the first invocation takes effect, and
    /// subsequent calls will return the already-initialized settings.
    ///
    /// See also:  
    /// <https://github.com/ethereum/c-kzg-4844?tab=readme-ov-file#precompute>
    ///
    /// # Arguments
    /// * `precompute` - Optional precompute value for KZG settings. If `None`,
    ///   defaults to `0`.
    ///
    /// # Returns
    /// A static reference to a `c_kzg::KzgSettings` instance.
    #[must_use]
    pub fn eth_kzg_settings(
        precompute: Option<u64>,
    ) -> &'static c_kzg::KzgSettings {
        const DEFAULT_PRECOMPUTE: u64 = 0;
        ethereum_kzg_settings(precompute.unwrap_or(DEFAULT_PRECOMPUTE))
    }

    /// Verifies a blob KZG proof.
    ///
    /// This function wraps the `verify_blob_kzg_proof` method from
    /// `c_kzg::KzgSettings` to validate a blob against its KZG commitment and
    /// proof. It is primarily used for Ethereum's EIP-4844 (blob-carrying
    /// transactions) verification.
    ///
    /// # Arguments
    /// * `settings` - A reference to the `c_kzg::KzgSettings` used for
    ///   verification.
    /// * `sidecar` - A `BlobSidecar` containing the blob data, commitment, and
    ///   proof to verify.
    ///
    /// # Returns
    /// * `Ok(true)` if the proof is valid.
    /// * `Ok(false)` if the proof is invalid.
    /// * `Err` if the blob, commitment, or proof cannot be parsed.
    ///
    /// # Errors
    /// Returns an error if the provided blob data, commitment, or proof
    /// cannot be converted to valid KZG types.
    pub fn verify_blob_kzg_proof(
        settings: &c_kzg::KzgSettings,
        sidecar: &BlobSidecar,
    ) -> Result<bool, Error> {
        let blob = KzgBlob::from_bytes(&sidecar.data)?;
        let commitment_bytes = Bytes48::new(sidecar.commitment);
        let proof_bytes = Bytes48::new(sidecar.proof);

        Ok(settings.verify_blob_kzg_proof(
            &blob,
            &commitment_bytes,
            &proof_bytes,
        )?)
    }

    /// Creates a `BlobData` from a byte slice containing the blob data part.
    ///
    /// This function also computes the KZG commitment and proof
    ///
    /// # Parameters
    /// - `data`: A byte slice containing the blob data part.
    /// - `precompute`: Optional precompute value for the KZG settings.
    ///
    /// # Errors
    /// If the blob data part is invalid or if there is an error in computing
    /// the KZG commitment or proof.
    pub fn from_datapart(
        data: &[u8],
        precompute: Option<u64>,
    ) -> Result<Self, Error> {
        let blob = KzgBlob::from_bytes(data)?;

        let settings = Self::eth_kzg_settings(precompute);

        let commitment = settings.blob_to_kzg_commitment(&blob)?.to_bytes();
        let proof = settings
            .compute_blob_kzg_proof(&blob, &commitment)?
            .to_bytes();
        let sidecar = BlobSidecar {
            commitment: commitment.into_inner(),
            proof: proof.into_inner(),
            data: blob.into_inner(),
        };
        let versioned_hash =
            BlobData::hash_from_commitment(&sidecar.commitment);

        Ok(Self {
            hash: versioned_hash,
            data: Some(sidecar),
        })
    }
}

impl From<c_kzg::Error> for Error {
    fn from(kzg_error: c_kzg::Error) -> Self {
        Self::Blob(kzg_error.to_string())
    }
}
