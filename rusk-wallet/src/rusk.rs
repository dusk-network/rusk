// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::io::{self, Write};

use reqwest::{Body, Response};
use rkyv::Archive;

use crate::Error;

/// Supported Rusk version
const REQUIRED_RUSK_VERSION: &str = "0.7.0";

#[derive(Debug)]
/// RuskRequesst according to the rusk event system
pub struct RuskRequest {
    topic: String,
    data: Vec<u8>,
}

impl RuskRequest {
    /// New RuskRequesst from topic and data
    pub fn new(topic: &str, data: Vec<u8>) -> Self {
        let topic = topic.to_string();
        Self { data, topic }
    }

    /// Return the binary representation of the RuskRequesst
    pub fn to_bytes(&self) -> io::Result<Vec<u8>> {
        let mut buffer = vec![];
        buffer.write_all(&(self.topic.len() as u32).to_le_bytes())?;
        buffer.write_all(self.topic.as_bytes())?;
        buffer.write_all(&self.data)?;

        Ok(buffer)
    }
}

#[derive(Clone)]
/// Rusk HTTP Binary Client
pub struct RuskHttpClient {
    uri: String,
}

impl RuskHttpClient {
    /// Create a new HTTP Client
    pub fn new(uri: String) -> Self {
        Self { uri }
    }

    /// Utility for querying the rusk VM
    pub async fn contract_query<I, const N: usize>(
        &self,
        contract: &str,
        method: &str,
        value: &I,
    ) -> Result<Vec<u8>, Error>
    where
        I: Archive,
        I: rkyv::Serialize<rkyv::ser::serializers::AllocSerializer<N>>,
    {
        let data = rkyv::to_bytes(value).map_err(|_| Error::Rkyv)?.to_vec();
        let request = RuskRequest::new(method, data);

        let response = self.call_raw(1, contract, &request, false).await?;

        Ok(response.bytes().await?.to_vec())
    }

    /// Check rusk connection
    pub async fn check_connection(&self) -> Result<(), reqwest::Error> {
        reqwest::Client::new().post(&self.uri).send().await?;
        Ok(())
    }

    /// Send a RuskRequest to a specific target.
    ///
    /// The response is interpreted as Binary
    pub async fn call(
        &self,
        target_type: u8,
        target: &str,
        request: &RuskRequest,
    ) -> Result<Vec<u8>, Error> {
        let response =
            self.call_raw(target_type, target, request, false).await?;
        let data = response.bytes().await?;
        Ok(data.to_vec())
    }
    /// Send a RuskRequest to a specific target without parsing the response
    pub async fn call_raw(
        &self,
        target_type: u8,
        target: &str,
        request: &RuskRequest,
        feed: bool,
    ) -> Result<Response, Error> {
        let uri = &self.uri;
        let client = reqwest::Client::new();
        let mut request = client
            .post(format!("{uri}/{target_type}/{target}"))
            .body(Body::from(request.to_bytes()?))
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
