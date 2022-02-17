// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use clap::{Arg, ArgMatches, Command};
use dusk_bytes::DeserializableSlice;
use dusk_pki::PublicSpendKey;
use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Serialize, Deserialize, Default, Clone)]
pub(crate) struct WalletConfig {
    generator: Option<String>,
}

fn parse_generator(key: &str) -> Option<PublicSpendKey> {
    bs58::decode(key)
        .into_vec()
        .ok()
        .and_then(|key| PublicSpendKey::from_slice(&key).ok())
}

impl WalletConfig {
    pub(crate) fn generator(&self) -> Option<PublicSpendKey> {
        parse_generator(self.generator.as_deref()?)
    }

    pub(crate) fn merge(&mut self, matches: &ArgMatches) {
        if let Some(key) = matches.value_of("generator") {
            self.generator =
                parse_generator(key).and(Some(key.to_string())).or_else(|| {
                    warn!(
                        "Failed parsing <generator>. Defaulting to Dusk's key"
                    );
                    None
                });
        }
    }

    pub fn inject_args(command: Command<'_>) -> Command<'_> {
        command.arg(
            Arg::new("generator")
                .long("generator")
                .value_name("Generator Key")
                .help("The public spend key in base58 that the block generator is going to be paid to")
                .takes_value(true),
        )
    }
}
