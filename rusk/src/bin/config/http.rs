// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::path::PathBuf;

use hyper::HeaderMap;
use serde::de::{self, Unexpected};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::args::Args;

#[derive(Serialize, Deserialize, Clone)]
pub struct HttpConfig {
    pub cert: Option<PathBuf>,
    pub key: Option<PathBuf>,
    #[serde(default = "default_listen")]
    pub listen: bool,
    #[serde(
        default = "default_feeder_call_gas",
        deserialize_with = "deserialize_feeder_call_gas",
        serialize_with = "serialize_feeder_call_gas"
    )]
    pub feeder_call_gas: u64,
    listen_address: Option<String>,
    #[serde(default = "default_ws_sub_channel_cap")]
    pub ws_sub_channel_cap: usize,
    #[serde(default = "default_ws_event_channel_cap")]
    pub ws_event_channel_cap: usize,
    #[serde(with = "vec_header_map", default = "default_http_headers")]
    pub headers: HeaderMap,
}

// Custom deserialization function for `feeder_call_gas`.
// TOML values are limited to `i64::MAX` in `toml-rs`, so we parse `u64` as a
// string.
fn deserialize_feeder_call_gas<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    String::deserialize(deserializer)?
        .parse::<u64>()
        .map_err(|_| {
            de::Error::invalid_value(
                Unexpected::Str("a valid u64 as a string"),
                &"a u64 integer",
            )
        })
}

// Custom serialization function for `feeder_call_gas`.
// Serializes `u64` as a string to bypass `i64::MAX` limitations in TOML
// parsing.
fn serialize_feeder_call_gas<S>(
    value: &u64,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            cert: None,
            key: None,
            headers: default_http_headers(),
            listen: default_listen(),
            feeder_call_gas: default_feeder_call_gas(),
            listen_address: None,
            ws_sub_channel_cap: default_ws_sub_channel_cap(),
            ws_event_channel_cap: default_ws_event_channel_cap(),
        }
    }
}

const fn default_feeder_call_gas() -> u64 {
    u64::MAX
}

const fn default_listen() -> bool {
    true
}

const fn default_ws_sub_channel_cap() -> usize {
    16
}

const fn default_ws_event_channel_cap() -> usize {
    1024
}

fn default_http_headers() -> HeaderMap {
    HeaderMap::new()
}

impl HttpConfig {
    pub fn listen_addr(&self) -> String {
        self.listen_address
            .clone()
            .unwrap_or("127.0.0.1:8080".into())
    }

    pub(crate) fn merge(&mut self, args: &Args) {
        // Overwrite config ws-listen-addr
        if let Some(http_listen_addr) = &args.http_listen_addr {
            self.listen_address = Some(http_listen_addr.into());
        }
    }
}

mod vec_header_map {
    use super::*;

    use std::fmt;

    use serde::de::{Deserializer, Error as _, SeqAccess, Visitor};
    use serde::ser::{Error as _, SerializeSeq, Serializer};

    use hyper::header::{HeaderName, HeaderValue};

    pub fn serialize<S>(
        headers: &HeaderMap,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let tuple_vec_len = headers.len();
        let mut tuple_vec = Vec::with_capacity(tuple_vec_len);

        for (k, v) in headers {
            let k_str = k.as_str();
            let v_str = v.to_str().map_err(S::Error::custom)?;

            tuple_vec.push((k_str, v_str));
        }

        let mut seq = serializer.serialize_seq(Some(tuple_vec_len))?;
        for elem in tuple_vec {
            seq.serialize_element(&elem)?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<HeaderMap, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TupleVecVisitor;

        impl<'de> Visitor<'de> for TupleVecVisitor {
            type Value = Vec<(String, String)>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a tuple header name and value")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let tuple_vec_len = seq.size_hint().unwrap_or_default();
                let mut tuple_vec = Vec::with_capacity(tuple_vec_len);

                while let Some(elem) = seq.next_element()? {
                    tuple_vec.push(elem);
                }

                Ok(tuple_vec)
            }
        }

        let tuple_vec = deserializer.deserialize_seq(TupleVecVisitor)?;

        let mut headers = HeaderMap::with_capacity(tuple_vec.len());
        for (k, v) in tuple_vec {
            let name = HeaderName::from_bytes(k.as_bytes())
                .map_err(D::Error::custom)?;
            let value = HeaderValue::from_str(&v).map_err(D::Error::custom)?;
            headers.insert(name, value);
        }

        Ok(headers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::http::HeaderValue;

    #[test]
    fn serialize_config() {
        let mut config = HttpConfig::default();

        config
            .headers
            .insert("name1", HeaderValue::from_str("value1").unwrap());
        config
            .headers
            .insert("name2", HeaderValue::from_str("value2").unwrap());

        let toml = toml::to_string(&config)
            .expect("serializing configuration should succeed");

        println!("{toml}");
    }

    #[test]
    fn deserialize_config() {
        let config_str = r#"listen = true
                            feeder_call_gas = "18446744"
                            ws_sub_channel_cap = 16
                            ws_event_channel_cap = 1024
                            headers = [["name1", "value1"], ["name2", "value2"]]"#;

        toml::from_str::<HttpConfig>(config_str)
            .expect("deserializing config should succeed");
    }

    #[test]
    fn deserialize_invalid_feeder_call_gas() {
        let config_str = r#"feeder_call_gas = "invalid_number""#;
        let result = toml::from_str::<HttpConfig>(config_str);
        assert!(result.is_err());

        let config_str = r#"feeder_call_gas = 18446744"#;
        let result = toml::from_str::<HttpConfig>(config_str);
        assert!(result.is_err());
    }
}
