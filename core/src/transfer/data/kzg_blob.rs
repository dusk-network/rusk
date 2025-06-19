// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Extra data that may be sent with the `data` field of either transaction
//! type.

use alloc::string::ToString;
use c_kzg::{ethereum_kzg_settings, Blob as KzgBlob};

use super::{BlobData, BlobSidecar, Error};

impl BlobData {
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

        let settings = {
            // https://github.com/ethereum/c-kzg-4844?tab=readme-ov-file#precompute
            const DEFAULT_PRECOMPUTE: u64 = 8;
            ethereum_kzg_settings(precompute.unwrap_or(DEFAULT_PRECOMPUTE))
        };

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
