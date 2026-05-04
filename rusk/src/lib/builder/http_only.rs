// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::time::Duration;

use tokio::sync::broadcast;
use tracing::info;

use crate::http::{DataSources, HandleRequest, HttpServer, HttpServerConfig};

pub struct RuskHttpBuilder {
    http: Option<HttpServerConfig>,
    data_sources: DataSources,
    shutdown_timeout: Duration,
}

impl RuskHttpBuilder {
    pub fn new() -> Self {
        Self {
            http: None,
            data_sources: DataSources::default(),
            shutdown_timeout: Duration::from_secs(30),
        }
    }

    pub fn with_http(mut self, http: HttpServerConfig) -> Self {
        self.http = Some(http);
        self
    }

    pub fn with_data_source(mut self, source: Box<dyn HandleRequest>) -> Self {
        self.data_sources.sources.push(source);
        self
    }

    pub fn with_shutdown_timeout(mut self, timeout: Duration) -> Self {
        self.shutdown_timeout = timeout;
        self
    }

    pub async fn build(self) -> anyhow::Result<RuskHttp> {
        let (_rues_sender, rues_receiver) = broadcast::channel(1);

        let mut server = None;
        if let Some(http) = self.http {
            info!("Configuring HTTP");

            #[allow(unused_mut)]
            let mut handler = self.data_sources;

            #[cfg(feature = "prover")]
            handler.sources.push(Box::new(rusk_prover::LocalProver));

            let cert_and_key = match (http.cert, http.key) {
                (Some(cert), Some(key)) => Some((cert, key)),
                _ => None,
            };

            let (http_server, _) = HttpServer::bind(
                handler,
                rues_receiver,
                http.ws_event_channel_cap,
                http.address,
                http.headers,
                cert_and_key,
            )
            .await?;

            server = Some(http_server);
        }

        Ok(RuskHttp {
            server,
            shutdown_timeout: self.shutdown_timeout,
        })
    }

    pub async fn build_and_run(self) -> anyhow::Result<()> {
        let mut http = self.build().await?;
        http.run().await
    }
}

impl Default for RuskHttpBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct RuskHttp {
    server: Option<HttpServer>,
    shutdown_timeout: Duration,
}

impl RuskHttp {
    pub async fn run(&mut self) -> anyhow::Result<()> {
        if let Some(server) = &mut self.server {
            server.wait().await?;
        }
        Ok(())
    }

    pub async fn shutdown(&mut self) -> anyhow::Result<()> {
        if let Some(server) = &mut self.server {
            tokio::time::timeout(self.shutdown_timeout, server.shutdown())
                .await
                .map_err(|_| {
                    anyhow::anyhow!(
                        "HTTP server failed to shut down within {} seconds",
                        self.shutdown_timeout.as_secs()
                    )
                })??;
        }
        Ok(())
    }
}
