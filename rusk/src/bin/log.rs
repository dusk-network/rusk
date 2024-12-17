// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use tracing::subscriber::SetGlobalDefaultError;
use tracing_subscriber::fmt::format::{DefaultFields, Format};
use tracing_subscriber::fmt::SubscriberBuilder;
use tracing_subscriber::EnvFilter;

pub struct Log {
    level: tracing::Level,
    filter: String,
    format: Option<String>,
}

impl Log {
    pub fn new(level: tracing::Level, filter: String) -> Self {
        Self {
            level,
            filter,
            format: None,
        }
    }

    pub fn with_format(mut self, format: String) -> Self {
        self.format = Some(format);
        self
    }

    fn subscriber(
        &self,
    ) -> SubscriberBuilder<DefaultFields, Format, EnvFilter> {
        // Generate a subscriber with the desired default log level and optional
        // log filter.
        tracing_subscriber::fmt::Subscriber::builder().with_env_filter(
            EnvFilter::new(self.filter.as_str())
                .add_directive(self.level.into()),
        )
    }

    pub fn register(self) -> Result<(), SetGlobalDefaultError> {
        match self.format.clone() {
            Some(format) => self.register_format(&format),
            None => self.register_simple(),
        }
    }

    #[allow(dead_code)]
    fn register_simple(self) -> Result<(), SetGlobalDefaultError> {
        let subscriber = self
            .subscriber()
            .with_level(false)
            .without_time()
            .with_target(false)
            .finish();
        tracing::subscriber::set_global_default(subscriber)
    }

    fn register_format(
        self,
        log_format: &str,
    ) -> Result<(), SetGlobalDefaultError> {
        let subscriber = self.subscriber();
        // Set the subscriber as global.
        // so this subscriber will be used as the default in all threads for the
        // remainder of the duration of the program, similar to how `loggers`
        // work in the `log` crate.
        match log_format {
            "json" => {
                let subscriber = subscriber
                    .json()
                    .with_current_span(false)
                    .flatten_event(true)
                    .finish();

                tracing::subscriber::set_global_default(subscriber)
            }
            "plain" => {
                let subscriber = subscriber.with_ansi(false).finish();
                tracing::subscriber::set_global_default(subscriber)
            }
            "coloured" => {
                let subscriber = subscriber.finish();
                tracing::subscriber::set_global_default(subscriber)
            }
            _ => unreachable!(),
        }
    }
}
