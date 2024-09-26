// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use tokio::sync::broadcast;
use tracing::info;

use crate::http::{DataSources, HttpServer, HttpServerConfig};

#[derive(Default)]
pub struct RuskHttpBuilder {
    http: Option<HttpServerConfig>,
}

impl RuskHttpBuilder {
    pub fn with_http(mut self, http: HttpServerConfig) -> Self {
        self.http = Some(http);
        self
    }

    pub async fn build_and_run(self) -> anyhow::Result<()> {
        let (_rues_sender, rues_receiver) = broadcast::channel(1);

        let mut _ws_server = None;
        if let Some(http) = self.http {
            info!("Configuring HTTP");

            #[allow(unused_mut)]
            let mut handler = DataSources::default();

            #[cfg(feature = "prover")]
            handler.sources.push(Box::new(rusk_prover::LocalProver));

            let cert_and_key = match (http.cert, http.key) {
                (Some(cert), Some(key)) => Some((cert, key)),
                _ => None,
            };

            _ws_server = Some(
                HttpServer::bind(
                    handler,
                    rues_receiver,
                    http.ws_event_channel_cap,
                    http.address,
                    http.headers,
                    cert_and_key,
                )
                .await?,
            );
        }

        if let Some(s) = _ws_server {
            s.wait().await?;
        }

        Ok(())
    }
}
