// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::time::{Duration, Instant};

use tracing::{debug, warn};

use super::*;

static CACHE: RwLock<(Option<Instant>, Vec<Value>)> =
    RwLock::const_new((None, Vec::new()));

impl RuskNode {
    pub(super) async fn peers_location(&self) -> ChainResult<ResponseData> {
        let locations = match from_cache().await {
            Some(locations) => locations,
            None => self.update_cache().await?,
        };

        Ok(ResponseData::new(serde_json::to_value(locations)?))
    }

    async fn update_cache(&self) -> ChainResult<Vec<Value>> {
        let mut cache = CACHE.write().await;
        if !cache_expired(cache.0) {
            return Ok(cache.1.clone());
        }

        let nodes = self.network().read().await.table().await;
        let mut locations = vec![];

        let client = reqwest::Client::new();

        let max_query = match std::env::var("IP_API_KEY") {
            Ok(_) => usize::MAX,
            Err(_) => 45,
        };

        let mut lookup_failures = 0usize;
        for n in nodes.iter().take(max_query) {
            let ip = n.ip();

            let url = match std::env::var("IP_API_KEY") {
                Ok(key) => {
                    format!("https://pro.ip-api.com/json/{ip}?key={key}")
                }
                Err(_) => format!("http://ip-api.com/json/{ip}"),
            };
            if let Ok(v) = client.get(url).send().await {
                let resp = v
                    .bytes()
                    .await
                    .map_err(|e| ChainError::internal(e.to_string()))?
                    .to_vec();
                let resp: Value = serde_json::from_slice(&resp)?;
                let mut object = Value::Object(Map::new());
                object["lat"] = resp["lat"].clone();
                object["lon"] = resp["lon"].clone();
                object["city"] = resp["city"].clone();
                object["country"] = resp["country"].clone();
                object["countryCode"] = resp["countryCode"].clone();
                locations.push(object);
            } else {
                // External geo lookup failures are best-effort and do not fail
                // the whole endpoint. We keep partial results instead.
                lookup_failures += 1;
            }
        }
        if lookup_failures > 0 {
            warn!(
                lookup_failures,
                resolved_locations = locations.len(),
                "Failed resolving some peer geo locations; returning partial results"
            );
        } else {
            debug!(
                resolved_locations = locations.len(),
                "Resolved peer geo locations"
            );
        }
        cache.0 = Some(Instant::now());
        cache.1 = locations;
        Ok(cache.1.clone())
    }
}

async fn from_cache() -> Option<Vec<Value>> {
    let cache = CACHE.read().await;
    if cache_expired(cache.0) {
        None
    } else {
        Some(cache.1.clone())
    }
}

fn cache_expired(last_update: Option<Instant>) -> bool {
    last_update
        .map(|i| i.elapsed() > Duration::from_secs(60))
        .unwrap_or(true)
}
