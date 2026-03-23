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
        let mut _ws_server = None;
        if let Some(http) = self.http {
            info!("Configuring HTTP");

            #[allow(unused_mut)]
            let mut handler = DataSources::default();

            #[cfg(feature = "prover")]
            handler.sources.push(Box::new(rusk_prover::LocalProver));

            // HTTP-only mode still needs a sender so routes can subscribe on
            // demand.
            let (rues_sender, _) = broadcast::channel(1);
            let (server, _) =
                HttpServer::bind(handler, rues_sender, http).await?;

            _ws_server = Some(server);
        }

        if let Some(s) = _ws_server {
            s.wait().await?;
        }

        Ok(())
    }
}
