// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use reqwest::{Body, Response};
use rkyv::Archive;

use crate::Error;

/// Supported Rusk version
const REQUIRED_RUSK_VERSION: &str = ">=0.8.0";

/// Target for contracts
pub const CONTRACTS_TARGET: &str = "contracts";

#[derive(Clone)]
/// Rusk HTTP Binary Client
pub struct RuesHttpClient {
    uri: String,
}

impl RuesHttpClient {
    /// Create a new HTTP Client
    pub fn new(uri: String) -> Self {
        Self { uri }
    }

    /// Utility for querying the rusk VM
    pub async fn contract_query<I, C, const N: usize>(
        &self,
        contract: C,
        method: &str,
        value: &I,
    ) -> Result<Vec<u8>, Error>
    where
        I: Archive,
        I: rkyv::Serialize<rkyv::ser::serializers::AllocSerializer<N>>,
        C: Into<Option<&'static str>>,
    {
        let data = rkyv::to_bytes(value).map_err(|_| Error::Rkyv)?.to_vec();

        let response = self
            .call_raw(CONTRACTS_TARGET, contract.into(), method, &data, false)
            .await?;

        Ok(response.bytes().await?.to_vec())
    }

    /// Check rusk connection
    /// Returns if status is success or not
    pub async fn check_connection(&self) -> Result<bool, reqwest::Error> {
        let request = reqwest::Client::new().post(&self.uri).send().await?;

        Ok(request.status().is_success())
    }

    /// Send a RuskRequest to a specific target.
    ///
    /// The response is interpreted as Binary
    pub async fn call<E>(
        &self,
        target: &str,
        entity: E,
        topic: &str,
        request: &[u8],
    ) -> Result<Vec<u8>, Error>
    where
        E: Into<Option<&'static str>>,
    {
        let response =
            self.call_raw(target, entity, topic, request, false).await?;
        let data = response.bytes().await?;
        Ok(data.to_vec())
    }

    /// Send a RuskRequest to a specific target without parsing the response
    pub async fn call_raw<E>(
        &self,
        target: &str,
        entity: E,
        topic: &str,
        data: &[u8],
        feed: bool,
    ) -> Result<Response, Error>
    where
        E: Into<Option<&'static str>>,
    {
        let uri = &self.uri;
        let client = reqwest::Client::new();
        let entity = entity.into().map(|e| format!(":{e}")).unwrap_or_default();

        let rues_prefix = if uri.ends_with('/') { "on" } else { "/on" };
        let mut request = client
            .post(format!("{uri}{rues_prefix}/{target}{entity}/{topic}"))
            .body(Body::from(data.to_vec()))
            .header("Content-Type", "application/octet-stream")
            .header("rusk-version", REQUIRED_RUSK_VERSION);

        if feed {
            request = request.header("Rusk-Feeder", "1");
        }
        let response = request.send().await?;

        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            let error = &response.bytes().await?;

            let error = String::from_utf8(error.to_vec())
                .unwrap_or("unparsable error".into());

            let msg = format!("{status}: {error}");

            Err(Error::Rusk(msg))
        } else {
            Ok(response)
        }
    }
}
