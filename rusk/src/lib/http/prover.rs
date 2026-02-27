// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_core::Error as CoreError;
use dusk_core::transfer::phoenix::Prove;
use rusk_prover::LocalProver;

use super::*;

type ProverApiResult<T> = Result<T, ProverApiError>;

#[derive(Debug, thiserror::Error)]
enum ProverApiError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Prover error: {0}")]
    Prover(String),
    #[error("Unsupported operation")]
    Unsupported,
}

impl ProverApiError {
    fn invalid_input<T: AsRef<str>>(msg: T) -> Self {
        Self::InvalidInput(msg.as_ref().to_string())
    }

    fn prover<T: AsRef<str>>(msg: T) -> Self {
        Self::Prover(msg.as_ref().to_string())
    }
}

impl From<ProverApiError> for HttpError {
    fn from(value: ProverApiError) -> Self {
        match value {
            ProverApiError::InvalidInput(msg) => HttpError::invalid_input(msg),
            ProverApiError::Prover(msg) => HttpError::prover(msg),
            ProverApiError::Unsupported => HttpError::Unsupported,
        }
    }
}

fn map_prove_error(error: CoreError) -> ProverApiError {
    match error {
        CoreError::InvalidData
        | CoreError::BadLength(_, _)
        | CoreError::InvalidChar(_, _)
        | CoreError::MemoTooLarge(_)
        | CoreError::Blob(_) => {
            ProverApiError::invalid_input(error.to_string())
        }
        _ => ProverApiError::prover(error.to_string()),
    }
}

#[async_trait]
impl HandleRequest for LocalProver {
    fn can_handle_rues(&self, request: &RuesDispatchEvent) -> bool {
        matches!(request.uri.inner(), ("prover", _, "prove"))
    }
    async fn handle_rues(
        &self,
        request: &RuesDispatchEvent,
    ) -> HttpResult<ResponseData> {
        let data = request.data.as_bytes();
        let response: ProverApiResult<ResponseData> = match request.uri.inner()
        {
            ("prover", _, "prove") => LocalProver
                .prove(data)
                .map(ResponseData::new)
                .map_err(map_prove_error),
            _ => Err(ProverApiError::Unsupported),
        };

        response.map_err(HttpError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::{HttpError, ProverApiError, map_prove_error};
    use dusk_core::Error as CoreError;

    #[test]
    fn prover_api_error_invalid_input_maps_to_http_invalid_input() {
        let http = HttpError::from(ProverApiError::invalid_input("bad input"));
        assert!(matches!(http, HttpError::InvalidInput(_)));
    }

    #[test]
    fn prover_api_error_prover_maps_to_http_prover() {
        let http = HttpError::from(ProverApiError::prover("prove failed"));
        assert!(matches!(http, HttpError::Prover(_)));
    }

    #[test]
    fn prover_api_error_unsupported_maps_to_http_unsupported() {
        let http = HttpError::from(ProverApiError::Unsupported);
        assert!(matches!(http, HttpError::Unsupported));
    }

    #[test]
    fn map_prove_error_invalid_data_maps_to_invalid_input() {
        let error = map_prove_error(CoreError::InvalidData);
        assert!(matches!(error, ProverApiError::InvalidInput(_)));
    }

    #[test]
    fn map_prove_error_prover_fault_maps_to_prover_error() {
        let error = map_prove_error(CoreError::PhoenixProver("x".into()));
        assert!(matches!(error, ProverApiError::Prover(_)));
    }
}
