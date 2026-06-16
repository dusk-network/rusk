// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::time::Duration;

use dusk_core::abi::ContractId;
use reqwest::{Body, Response};
use rkyv::Archive;
use url::Url;

use crate::{Error, RuskError};

/// Supported Rusk version
const REQUIRED_RUSK_VERSION: &str = "1.0.0-rc.0";
const GRAPHQL_ACCEPT: &str =
    "application/graphql-response+json, application/json";
const GRAPHQL_PING_QUERY: &str = "query { __typename }";

/// Target for contracts
pub const CONTRACTS_TARGET: &str = "contracts";

fn normalize_base_node_url(uri: &str) -> Result<Url, Error> {
    let mut url = Url::parse(uri).map_err(|source| {
        Error::Rusk(RuskError::InvalidBaseNodeUri {
            uri: uri.to_string(),
            source,
        })
    })?;
    url.set_query(None);
    url.set_fragment(None);

    let path = url.path().trim_end_matches('/');
    let path = if path.is_empty() {
        "/".to_string()
    } else {
        format!("{path}/")
    };
    url.set_path(&path);
    Ok(url)
}

fn has_graphql_errors(errors: &serde_json::Value) -> bool {
    match errors {
        serde_json::Value::Array(entries) => !entries.is_empty(),
        serde_json::Value::Null => false,
        _ => true,
    }
}

/// Encode a typed contract id into the Rues entity string used on HTTP paths.
#[must_use]
fn contract_entity(contract: &ContractId) -> String {
    hex::encode(contract.as_bytes())
}

#[derive(Clone)]
/// Rusk HTTP Binary Client
pub struct HttpClient {
    client: reqwest::Client,
    base_url: Url,
}

impl HttpClient {
    /// Create a new HTTP Client
    ///
    /// # Arguments
    /// - `uri` - Base node URL, for example `http://127.0.0.1:8080`, `http://127.0.0.1:8080/`,
    ///   or `https://proxy.example.com/rusk`.
    ///
    /// # Errors
    /// This method errors if a TLS backend cannot be initialized, or the
    /// resolver cannot load the system configuration.
    pub fn new<S: Into<String>>(uri: S) -> Result<Self, Error> {
        let uri = uri.into();
        let base_url = normalize_base_node_url(&uri)?;
        let client = reqwest::ClientBuilder::new()
            .connect_timeout(Duration::from_secs(30))
            .build();

        match client {
            Ok(client) => Ok(Self { client, base_url }),
            Err(_) => Err(Error::HttpClient),
        }
    }

    fn endpoint(&self, path: &str) -> Result<Url, Error> {
        self.base_url.join(path).map_err(|source| {
            Error::Rusk(RuskError::InvalidEndpointPath {
                path: path.to_string(),
                source,
            })
        })
    }

    /// Utility for querying the rusk VM
    ///
    /// # Errors
    /// This method errors if there was an error while sending the rues request,
    /// if the response body is not in JSON format or if the value cannot be
    /// serialized using rkyv.
    pub async fn contract_query<I, const N: usize>(
        &self,
        contract: ContractId,
        method: &str,
        value: &I,
    ) -> Result<Vec<u8>, Error>
    where
        I: Archive,
        I: rkyv::Serialize<rkyv::ser::serializers::AllocSerializer<N>>,
    {
        let data = rkyv::to_bytes(value).map_err(|_| Error::Rkyv)?.to_vec();

        let response = self
            .call_contract_raw(CONTRACTS_TARGET, contract, method, &data, false)
            .await?;

        Ok(response.bytes().await?.to_vec())
    }

    /// Check rusk connection
    ///
    /// # Errors
    /// This method errors if there was an error while sending the request.
    pub async fn check_connection(&self) -> Result<(), reqwest::Error> {
        self.client.post(self.base_url.clone()).send().await?;

        Ok(())
    }

    /// Check GraphQL connection through the standard `/graphql` endpoint.
    ///
    /// # Errors
    /// This method errors if there was an error while sending the request,
    /// or if GraphQL returned an error response.
    pub async fn check_graphql_connection(&self) -> Result<(), Error> {
        self.graphql_query(GRAPHQL_PING_QUERY).await.map(|_| ())
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

    /// Send a `RuskRequest` to a contract-backed target without parsing the
    /// response.
    ///
    /// # Errors
    /// This method errors if there was an error while sending the rues request,
    /// or if the response body is not in JSON format.
    pub(crate) async fn call_contract_raw(
        &self,
        target: &str,
        contract: ContractId,
        topic: &str,
        data: &[u8],
        feed: bool,
    ) -> Result<Response, Error> {
        let entity = contract_entity(&contract);

        self.send_request(target, Some(entity.as_str()), topic, data, feed)
            .await
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
        self.send_request(target, entity.into(), topic, data, feed)
            .await
    }

    async fn send_request(
        &self,
        target: &str,
        entity: Option<&str>,
        topic: &str,
        data: &[u8],
        feed: bool,
    ) -> Result<Response, Error> {
        let entity = entity
            .map(|entity| format!(":{entity}"))
            .unwrap_or_default();
        let endpoint =
            self.endpoint(&format!("on/{target}{entity}/{topic}"))?;
        let mut request = self
            .client
            .post(endpoint)
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

            Err(Error::Rusk(RuskError::Response { message: msg }))
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
        let endpoint = self.endpoint(&format!(
            "on/contract:{}/upload_driver",
            hex::encode(contract_id.as_bytes())
        ))?;
        let request = self
            .client
            .post(endpoint)
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

            Err(Error::Rusk(RuskError::Response { message: msg }))
        } else {
            let data = response.bytes().await?;
            Ok(data.to_vec())
        }
    }

    /// Send a GraphQL query to the `/graphql` endpoint and return
    /// the unwrapped `data` payload.
    ///
    /// # Errors
    ///
    /// This method errors if there was an error while sending the request, if
    /// the response body is not in JSON format, if the GraphQL response
    /// contains an `errors` field, or if the `data` field is missing from
    /// the GraphQL response.
    pub async fn graphql_query(&self, query: &str) -> Result<Vec<u8>, Error> {
        let endpoint = self.endpoint("graphql")?;
        let query = if query.trim().is_empty() {
            GRAPHQL_PING_QUERY
        } else {
            query
        };

        let response = self
            .client
            .post(endpoint)
            .header("Accept", GRAPHQL_ACCEPT)
            .header("Content-Type", "application/json")
            .header("rusk-version", REQUIRED_RUSK_VERSION)
            .body(serde_json::json!({ "query": query }).to_string())
            .send()
            .await?;

        let status = response.status();
        let body = response.bytes().await?;

        if status.is_client_error() || status.is_server_error() {
            let error = String::from_utf8(body.to_vec())
                .unwrap_or_else(|_| "unparsable error".into());
            return Err(Error::Rusk(RuskError::Response { message: error }));
        }

        let payload: serde_json::Value = serde_json::from_slice(&body)?;
        if let Some(errors) = payload.get("errors")
            && has_graphql_errors(errors)
        {
            return Err(Error::Rusk(RuskError::GraphqlErrors {
                errors: errors.clone(),
            }));
        }

        match payload.get("data") {
            Some(data) => serde_json::to_vec(data).map_err(Error::from),
            None => Err(Error::Rusk(RuskError::MissingGraphqlData)),
        }
    }
}

#[cfg(test)]
mod tests {
    use dusk_core::abi::ContractId;
    use dusk_core::stake::STAKE_CONTRACT;
    use dusk_core::transfer::TRANSFER_CONTRACT;

    use super::{
        HttpClient, contract_entity, has_graphql_errors,
        normalize_base_node_url,
    };

    #[test]
    fn contract_entity_matches_contract_bytes_hex_encoding() {
        let custom_contract = ContractId::from([0xabu8; 32]);

        assert_eq!(
            contract_entity(&TRANSFER_CONTRACT),
            hex::encode(TRANSFER_CONTRACT.as_bytes())
        );
        assert_eq!(
            contract_entity(&STAKE_CONTRACT),
            hex::encode(STAKE_CONTRACT.as_bytes())
        );
        assert_eq!(
            contract_entity(&custom_contract),
            hex::encode(custom_contract.as_bytes())
        );
    }

    #[test]
    fn normalize_base_node_url_accepts_root_paths() {
        let url = normalize_base_node_url("http://127.0.0.1:8080").unwrap();
        assert_eq!(url.as_str(), "http://127.0.0.1:8080/");

        let url = normalize_base_node_url("http://127.0.0.1:8080/").unwrap();
        assert_eq!(url.as_str(), "http://127.0.0.1:8080/");
    }

    #[test]
    fn http_client_derives_prefixed_graphql_and_rues_endpoints() {
        let url = normalize_base_node_url("https://example.com/rusk").unwrap();
        assert_eq!(url.as_str(), "https://example.com/rusk/");

        let url = normalize_base_node_url("https://example.com/rusk/").unwrap();
        assert_eq!(url.as_str(), "https://example.com/rusk/");

        let client = HttpClient::new("https://example.com/rusk").unwrap();

        assert_eq!(client.base_url.as_str(), "https://example.com/rusk/");
        assert_eq!(
            client.endpoint("graphql").unwrap().as_str(),
            "https://example.com/rusk/graphql"
        );
        assert_eq!(
            client
                .endpoint("on/contracts:deadbeef/query")
                .unwrap()
                .as_str(),
            "https://example.com/rusk/on/contracts:deadbeef/query"
        );
    }

    #[test]
    fn has_graphql_errors_ignores_empty_arrays_and_null() {
        assert!(!has_graphql_errors(&serde_json::json!([])));
        assert!(!has_graphql_errors(&serde_json::Value::Null));
    }

    #[test]
    fn has_graphql_errors_flags_non_empty_values() {
        assert!(has_graphql_errors(&serde_json::json!([
            { "message": "boom" }
        ])));
        assert!(has_graphql_errors(
            &serde_json::json!({ "message": "boom" })
        ));
    }
}
