// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fmt;
use std::path::PathBuf;

use rusk_wallet::Error;
use serde::Deserialize;
use tracing::Level;
use url::{Host, Url};
use zeroize::Zeroize;

use crate::config::Network;
use crate::io::WalletArgs;

#[derive(clap::ValueEnum, Debug, Clone)]
pub(crate) enum LogFormat {
    Json,
    Plain,
    Coloured,
}

#[derive(clap::ValueEnum, Debug, Clone)]
pub(crate) enum LogLevel {
    /// Designates very low priority, often extremely verbose, information.
    Trace,
    /// Designates lower priority information.
    Debug,
    /// Designates useful information.
    Info,
    /// Designates hazardous situations.
    Warn,
    /// Designates very serious errors.
    Error,
}

#[derive(Debug)]
pub(crate) struct Logging {
    /// Max log level
    pub level: LogLevel,
    /// Log format
    pub format: LogFormat,
}

#[derive(Debug)]
pub(crate) struct Settings {
    pub(crate) network_name: Option<String>,
    pub(crate) state: Url,
    pub(crate) prover: Url,
    pub(crate) archiver: Url,
    pub(crate) explorer: Option<Url>,
    pub(crate) allow_insecure: bool,

    pub(crate) logging: Logging,

    pub(crate) wallet_dir: PathBuf,
    pub(crate) password: Option<String>,
}

pub(crate) struct SettingsBuilder {
    wallet_dir: PathBuf,
    pub(crate) args: WalletArgs,
}

impl SettingsBuilder {
    pub fn wallet_dir(&self) -> &PathBuf {
        &self.wallet_dir
    }

    pub fn network(self, network: Network) -> Result<Settings, Error> {
        let mut args = self.args;
        let selected_network_name = args.network.clone();

        let network =
            match (selected_network_name.clone(), network.clone().network) {
                (Some(label), Some(mut networks)) => {
                    let r = networks.remove(&label);
                    // err if specified network is not in the list
                    if r.is_none() {
                        args.password.zeroize();
                        return Err(Error::NetworkNotFound);
                    }

                    r
                }
                // err if no networks are specified but argument is
                (Some(_), None) => {
                    args.password.zeroize();
                    return Err(Error::NetworkNotFound);
                }
                (_, _) => None,
            }
            .unwrap_or(network);

        let state = parse_endpoint_override(
            "state",
            args.state.as_deref(),
            network.state,
        )
        .inspect_err(|_| args.password.zeroize())?;

        let prover = parse_endpoint_override(
            "prover",
            args.prover.as_deref(),
            network.prover,
        )
        .inspect_err(|_| args.password.zeroize())?;

        let archiver = parse_endpoint_override(
            "archiver",
            args.archiver.as_deref(),
            network.archiver.unwrap_or_else(|| state.clone()),
        )
        .inspect_err(|_| args.password.zeroize())?;

        let explorer = network.explorer;
        let allow_insecure = args.allow_insecure;

        validate_transport("state", &state, allow_insecure)
            .inspect_err(|_| args.password.zeroize())?;
        validate_transport("prover", &prover, allow_insecure)
            .inspect_err(|_| args.password.zeroize())?;
        validate_transport("archiver", &archiver, allow_insecure)
            .inspect_err(|_| args.password.zeroize())?;

        let wallet_dir =
            args.wallet_dir.as_ref().cloned().unwrap_or(self.wallet_dir);

        let password = args.password;

        let logging = Logging {
            level: args.log_level,
            format: args.log_type,
        };

        Ok(Settings {
            network_name: selected_network_name,
            state,
            prover,
            archiver,
            explorer,
            allow_insecure,
            logging,
            wallet_dir,
            password,
        })
    }
}

fn parse_endpoint_override(
    endpoint: &'static str,
    value: Option<&str>,
    default: Url,
) -> Result<Url, Error> {
    match value {
        Some(value) => {
            Url::parse(value).map_err(|source| Error::InvalidEndpointUrl {
                endpoint,
                value: value.to_string(),
                source,
            })
        }
        None => Ok(default),
    }
}

fn validate_transport(
    endpoint: &'static str,
    url: &Url,
    allow_insecure: bool,
) -> Result<(), Error> {
    match url.scheme() {
        "https" => Ok(()),
        "http" if is_local_host(url) || allow_insecure => Ok(()),
        "http" => Err(Error::InsecureTransport(format!(
            "{endpoint} endpoint {url}"
        ))),
        scheme => Err(Error::UnsupportedEndpointScheme {
            endpoint,
            scheme: scheme.to_string(),
            value: url.to_string(),
        }),
    }
}

fn is_local_host(url: &Url) -> bool {
    match url.host() {
        Some(Host::Domain("localhost")) => true,
        Some(Host::Ipv4(ip)) => ip.is_loopback(),
        Some(Host::Ipv6(ip)) => ip.is_loopback(),
        _ => false,
    }
}

impl Settings {
    pub fn args(args: WalletArgs) -> Result<SettingsBuilder, Error> {
        let wallet_dir = if let Some(path) = &args.wallet_dir {
            path.clone()
        } else if let Some(path) = get_wallet_dir_config_from_exe_dir() {
            path
        } else {
            let mut path = dirs::home_dir().ok_or(Error::OsNotSupported)?;
            path.push(".dusk");
            path.push(env!("CARGO_BIN_NAME"));
            path
        };

        Ok(SettingsBuilder { wallet_dir, args })
    }

    pub fn nonlocal_insecure_endpoints(&self) -> Vec<(&'static str, &Url)> {
        let mut endpoints = Vec::new();

        if self.state.scheme() == "http" && !is_local_host(&self.state) {
            endpoints.push(("state", &self.state));
        }

        if self.prover.scheme() == "http" && !is_local_host(&self.prover) {
            endpoints.push(("prover", &self.prover));
        }

        if self.archiver.scheme() == "http" && !is_local_host(&self.archiver) {
            endpoints.push(("archiver", &self.archiver));
        }

        endpoints
    }
}

#[derive(Deserialize, Debug)]
struct WalletDirConfig {
    wallet_dir: PathBuf,
}

fn get_wallet_dir_config_from_exe_dir() -> Option<PathBuf> {
    let mut path = std::env::current_exe().ok()?;
    path.pop();
    let config_path = path.join("config.toml");
    if !config_path.exists() {
        return None;
    }
    let contents = std::fs::read_to_string(config_path).ok()?;
    let wallet_dir_config: WalletDirConfig = toml::from_str(&contents).ok()?;
    Some(wallet_dir_config.wallet_dir)
}

impl From<&LogLevel> for Level {
    fn from(level: &LogLevel) -> Level {
        match level {
            LogLevel::Trace => Level::TRACE,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Info => Level::INFO,
            LogLevel::Warn => Level::WARN,
            LogLevel::Error => Level::ERROR,
        }
    }
}

impl fmt::Display for LogFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Json => "json",
                Self::Plain => "plain",
                Self::Coloured => "coloured",
            }
        )
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Trace => "trace",
                Self::Debug => "debug",
                Self::Info => "info",
                Self::Warn => "warn",
                Self::Error => "error",
            }
        )
    }
}

impl fmt::Display for Logging {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Logging: [{}] ({})", self.level, self.format)
    }
}

impl fmt::Display for Settings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let separator = "─".repeat(14);
        writeln!(f, "{separator}")?;
        writeln!(f, "Settings")?;
        writeln!(f, "{separator}")?;
        writeln!(f, "Wallet directory: {}", self.wallet_dir.display())?;
        writeln!(
            f,
            "Password: {}",
            if self.password.is_some() {
                "[Set]"
            } else {
                "[Not set]"
            }
        )?;
        writeln!(f, "{separator}")?;
        if let Some(network_name) = &self.network_name {
            writeln!(f, "network: {network_name}")?;
        }
        writeln!(f, "state: {}", self.state)?;
        writeln!(f, "prover: {}", self.prover)?;
        writeln!(
            f,
            "allow insecure transport: {}",
            if self.allow_insecure { "yes" } else { "no" }
        )?;

        if let Some(explorer) = &self.explorer {
            writeln!(f, "explorer: {explorer}")?;
        }

        writeln!(f, "{separator}")?;
        writeln!(f, "{}", self.logging)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::io::WalletArgs;

    fn wallet_args() -> WalletArgs {
        WalletArgs {
            wallet_dir: None,
            network: None,
            password: None,
            state: None,
            prover: None,
            archiver: None,
            allow_insecure: false,
            log_level: LogLevel::Info,
            log_type: LogFormat::Coloured,
            command: None,
        }
    }

    fn network(state: &str, prover: &str, archiver: Option<&str>) -> Network {
        Network {
            state: Url::parse(state).unwrap(),
            prover: Url::parse(prover).unwrap(),
            archiver: archiver.map(|url| Url::parse(url).unwrap()),
            explorer: None,
            network: None,
        }
    }

    #[test]
    fn rejects_remote_http_without_opt_in() {
        let wallet_dir = tempdir().unwrap();
        let builder = SettingsBuilder {
            wallet_dir: wallet_dir.path().to_path_buf(),
            args: wallet_args(),
        };

        let err = builder
            .network(network(
                "http://example.com:8080",
                "https://prover.example.com",
                None,
            ))
            .unwrap_err();

        assert!(matches!(err, Error::InsecureTransport(_)));
    }

    #[test]
    fn allows_local_http_without_opt_in() {
        let wallet_dir = tempdir().unwrap();
        let builder = SettingsBuilder {
            wallet_dir: wallet_dir.path().to_path_buf(),
            args: wallet_args(),
        };

        let settings = builder
            .network(network(
                "http://127.0.0.1:8080",
                "http://[::1]:8080",
                Some("http://localhost:8080"),
            ))
            .unwrap();

        assert!(!settings.allow_insecure);
    }

    #[test]
    fn allows_ipv6_loopback_http_without_opt_in() {
        let wallet_dir = tempdir().unwrap();
        let builder = SettingsBuilder {
            wallet_dir: wallet_dir.path().to_path_buf(),
            args: wallet_args(),
        };

        let settings = builder
            .network(network(
                "http://[::1]:8080",
                "https://prover.example.com",
                None,
            ))
            .unwrap();

        assert!(!settings.allow_insecure);
    }

    #[test]
    fn rejects_ipv4_unspecified_http_without_opt_in() {
        let wallet_dir = tempdir().unwrap();
        let builder = SettingsBuilder {
            wallet_dir: wallet_dir.path().to_path_buf(),
            args: wallet_args(),
        };

        let err = builder
            .network(network(
                "http://0.0.0.0:8080",
                "https://prover.example.com",
                None,
            ))
            .unwrap_err();

        assert!(matches!(err, Error::InsecureTransport(_)));
    }

    #[test]
    fn rejects_ipv6_unspecified_http_without_opt_in() {
        let wallet_dir = tempdir().unwrap();
        let builder = SettingsBuilder {
            wallet_dir: wallet_dir.path().to_path_buf(),
            args: wallet_args(),
        };

        let err = builder
            .network(network(
                "http://[::]:8080",
                "https://prover.example.com",
                None,
            ))
            .unwrap_err();

        assert!(matches!(err, Error::InsecureTransport(_)));
    }

    #[test]
    fn rejects_unsupported_endpoint_scheme() {
        let wallet_dir = tempdir().unwrap();
        let builder = SettingsBuilder {
            wallet_dir: wallet_dir.path().to_path_buf(),
            args: wallet_args(),
        };

        let err = builder
            .network(network(
                "ftp://example.com:8080",
                "https://prover.example.com",
                None,
            ))
            .unwrap_err();

        assert!(matches!(
            err,
            Error::UnsupportedEndpointScheme {
                endpoint: "state",
                ..
            }
        ));
    }

    #[test]
    fn allows_remote_http_with_opt_in() {
        let mut args = wallet_args();
        args.allow_insecure = true;

        let wallet_dir = tempdir().unwrap();
        let builder = SettingsBuilder {
            wallet_dir: wallet_dir.path().to_path_buf(),
            args,
        };

        let settings = builder
            .network(network(
                "http://example.com:8080",
                "http://prover.example.com:8080",
                Some("http://archiver.example.com:8080"),
            ))
            .unwrap();

        assert!(settings.allow_insecure);
    }

    #[test]
    fn rejects_invalid_state_override_url() {
        let mut args = wallet_args();
        args.state = Some("not a url".into());

        let wallet_dir = tempdir().unwrap();
        let builder = SettingsBuilder {
            wallet_dir: wallet_dir.path().to_path_buf(),
            args,
        };

        let err = builder
            .network(network(
                "https://state.example.com",
                "https://prover.example.com",
                None,
            ))
            .unwrap_err();

        assert!(matches!(
            err,
            Error::InvalidEndpointUrl {
                endpoint: "state",
                ..
            }
        ));
    }

    #[test]
    fn rejects_invalid_archiver_override_url() {
        let mut args = wallet_args();
        args.archiver = Some(":://broken".into());

        let wallet_dir = tempdir().unwrap();
        let builder = SettingsBuilder {
            wallet_dir: wallet_dir.path().to_path_buf(),
            args,
        };

        let err = builder
            .network(network(
                "https://state.example.com",
                "https://prover.example.com",
                Some("https://archiver.example.com"),
            ))
            .unwrap_err();

        assert!(matches!(
            err,
            Error::InvalidEndpointUrl {
                endpoint: "archiver",
                ..
            }
        ));
    }

    #[test]
    fn nonlocal_insecure_endpoints_skip_local_urls() {
        let wallet_dir = tempdir().unwrap();
        let settings = Settings {
            network_name: None,
            state: Url::parse("http://0.0.0.0:8080").unwrap(),
            prover: Url::parse("http://prover.example.com:8080").unwrap(),
            archiver: Url::parse("http://localhost:8080").unwrap(),
            explorer: None,
            allow_insecure: true,
            logging: Logging {
                level: LogLevel::Info,
                format: LogFormat::Coloured,
            },
            wallet_dir: wallet_dir.path().to_path_buf(),
            password: None,
        };

        let endpoints = settings.nonlocal_insecure_endpoints();

        assert_eq!(endpoints.len(), 2);
        assert_eq!(endpoints[0].0, "state");
        assert_eq!(endpoints[0].1.as_str(), "http://0.0.0.0:8080/");
        assert_eq!(endpoints[1].0, "prover");
        assert_eq!(endpoints[1].1.as_str(), "http://prover.example.com:8080/");
    }
}
