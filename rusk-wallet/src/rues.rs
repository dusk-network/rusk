// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::time::Duration;

use dusk_core::abi::ContractId;
use reqwest::{Body, Response};
use rkyv::Archive;

use crate::Error;

/// Supported Rusk version
const REQUIRED_RUSK_VERSION: &str = "1.0.0-rc.0";

/// Target for contracts
pub const CONTRACTS_TARGET: &str = "contracts";

#[derive(Clone)]
/// Rusk HTTP Binary Client
pub struct HttpClient {
    client: reqwest::Client,
    uri: String,
}

impl HttpClient {
    /// Create a new HTTP Client
    ///
    /// # Errors
    /// This method errors if a TLS backend cannot be initialized, or the
    /// resolver cannot load the system configuration.
    pub fn new<S: Into<String>>(uri: S) -> Result<Self, Error> {
        let client = reqwest::ClientBuilder::new()
            .connect_timeout(Duration::from_secs(30))
            .build();

        match client {
            Ok(client) => Ok(Self {
                uri: uri.into(),
                client,
            }),
            Err(_) => Err(Error::HttpClient),
        }
    }

    /// Utility for querying the rusk VM
    ///
    /// # Errors
    /// This method errors if there was an error while sending the rues request,
    /// if the response body is not in JSON format or if the value cannot be
    /// serialized using rkyv.
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
    ///
    /// # Errors
    /// This method errors if there was an error while sending the request.
    pub async fn check_connection(&self) -> Result<(), reqwest::Error> {
        self.client.post(&self.uri).send().await?;

        Ok(())
    }

    /// Send a `RuskRequest` to a specific target.
    ///
    /// The response is interpreted as Binary
    ///
    /// # Errors
    /// This method errors if there was an error while sending the request,
    /// or if the response body is not in JSON format.
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

    /// Send a `RuskRequest` to a specific target without parsing the response
    ///
    /// # Errors
    /// This method errors if there was an error while sending the rues request,
    /// or if the response body is not in JSON format.
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
        let entity = entity.into().map(|e| format!(":{e}")).unwrap_or_default();

        let rues_prefix = if uri.ends_with('/') { "on" } else { "/on" };
        let mut request = self
            .client
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

            let msg = if error.contains("Value spent larger than account holds")
            {
                "Balance is not enough to cover the transaction max fees".into()
            } else {
                error
            };

            Err(Error::Rusk(msg))
        } else {
            Ok(response)
        }
    }

    /// Upload the data driver
    ///
    /// # Errors
    /// When signer is not the owner of a contract indicated by owner id
    /// When signed hash is not a hash of the uploaded bytecode
    /// When contract is not deployed
    pub async fn upload_driver(
        &self,
        driver_bytecode: impl AsRef<[u8]>,
        contract_id: &ContractId,
        signature: impl AsRef<[u8]>,
    ) -> Result<Vec<u8>, Error> {
        let uri = &self.uri;
        let client = reqwest::Client::new();
        let target = "contract";
        let entity = hex::encode(contract_id.as_bytes());
        let entity = if entity.is_empty() {
            entity.to_string()
        } else {
            format!(":{entity}")
        };
        let topic = "upload_driver";
        let rues_prefix = if uri.ends_with('/') { "on" } else { "/on" };
        let request = client
            .post(format!("{uri}{rues_prefix}/{target}{entity}/{topic}"))
            .body(Body::from(driver_bytecode.as_ref().to_vec()))
            .header("Content-Type", "application/octet-stream")
            .header("rusk-version", REQUIRED_RUSK_VERSION)
            .header("sign", hex::encode(signature.as_ref()));

        let response = request.send().await?;

        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            let error = &response.bytes().await?;

            let error = String::from_utf8(error.to_vec())
                .unwrap_or("unparsable error".into());

            let msg = format!("{status}: {error}");

            Err(Error::Rusk(msg))
        } else {
            let data = response.bytes().await?;
            Ok(data.to_vec())
        }
    }
}
