// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use super::error::ChainResult;
use super::*;
use std::time::{Duration, Instant};

static CACHE: RwLock<(Option<Instant>, Vec<Value>)> =
    RwLock::const_new((None, Vec::new()));

impl RuskNode {
    pub async fn peers_location(&self) -> ChainResult<ResponseData> {
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

        let mut nodes = self.network().read().await.table().await;
        let mut locations = vec![];

        let client = reqwest::Client::new();

        let max_query = match std::env::var("IP_API_KEY") {
            Ok(_) => usize::MAX,
            Err(_) => 45,
        };

        for n in nodes.iter().take(max_query) {
            let ip = n.ip();

            let url = match std::env::var("IP_API_KEY") {
                Ok(key) => {
                    format!("https://pro.ip-api.com/json/{ip}?key={key}")
                }
                Err(_) => format!("http://ip-api.com/json/{ip}"),
            };
            if let Ok(v) = client.get(url).send().await {
                let resp = v.bytes().await?.to_vec();
                let mut resp: Value = serde_json::from_slice(&resp)?;
                let mut object = Value::Object(Map::new());
                object["lat"] = resp["lat"].clone();
                object["lon"] = resp["lon"].clone();
                object["city"] = resp["city"].clone();
                object["country"] = resp["country"].clone();
                object["countryCode"] = resp["countryCode"].clone();
                locations.push(object);
            }
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
