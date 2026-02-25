// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::sync::{LazyLock, Mutex, mpsc};
use std::{cmp, time::Duration};
use std::{io, thread};

use dusk_core::transfer::phoenix::TRANSCRIPT_LABEL;
use dusk_plonk::prelude::{Compiler, PublicParameters};
use reqwest::StatusCode;
use reqwest::header::RETRY_AFTER;
use rusk_profile::Circuit as CircuitProfile;
use tokio::time::sleep;
use tracing::{info, warn};

use crate::Theme;

mod circuits;

const MAX_CRS_DOWNLOAD_ATTEMPTS: usize = 8;
const BASE_RETRY_DELAY_MS: u64 = 500;
const MAX_RETRY_DELAY_SECS: u64 = 30;

static CRS_URL: LazyLock<Mutex<String>> =
    LazyLock::new(|| Mutex::new(String::new()));

static PUB_PARAMS: LazyLock<PublicParameters> = LazyLock::new(|| {
    let theme = Theme::default();
    info!("{} CRS from cache", theme.action("Fetching"));
    match rusk_profile::get_common_reference_string() {
        Ok(buff) if rusk_profile::verify_common_reference_string(&buff) => {
            let pp = PublicParameters::from_slice(&buff[..])
                .expect("Creating PublicParameters from slice failed.");
            info!("{} CRS", theme.info("Loaded"));
            pp
        }

        _ => {
            warn!(
                "{} CRS from server due to cache miss",
                theme.warn("Fetching")
            );
            let (tx, rx) = mpsc::channel();

            thread::spawn(move || {
                let pp_bytes =
                    fetch_pp().expect("PublicParameters download failed.");
                tx.send(pp_bytes).unwrap();
            })
            .join()
            .expect("PublicParameters download thread panicked");

            let pp_bytes = rx.recv().unwrap();
            let pp = PublicParameters::from_slice(pp_bytes.as_slice())
                .expect("Creating PublicParameters from slice failed.");

            rusk_profile::set_common_reference_string(pp_bytes)
                .expect("Unable to write the CRS");

            info!("{} CRS", theme.info("Cached"));

            pp
        }
    }
});

#[tokio::main]
async fn fetch_pp() -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let crs_url = CRS_URL.lock().expect("Unlocking failed.").to_string();
    let client = reqwest::Client::new();
    let mut last_failure = String::new();

    for attempt in 1..=MAX_CRS_DOWNLOAD_ATTEMPTS {
        match client.get(&crs_url).send().await {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    let bytes = response.bytes().await?;
                    return Ok(bytes.to_vec());
                }

                let retry_after = parse_retry_after_header(
                    response.headers().get(RETRY_AFTER),
                );
                let body = response.text().await.unwrap_or_else(|_| {
                    String::from("<response body unavailable>")
                });
                last_failure = format!("status: {status}; body: {body}");

                if should_retry_status(status)
                    && attempt < MAX_CRS_DOWNLOAD_ATTEMPTS
                {
                    let delay = retry_after
                        .unwrap_or_else(|| exponential_backoff_delay(attempt));
                    warn!(
                        "CRS download failed ({status}), retrying in {:?} (attempt {attempt}/{MAX_CRS_DOWNLOAD_ATTEMPTS})",
                        delay
                    );
                    sleep(delay).await;
                    continue;
                }

                return Err(io::Error::other(format!(
                    "failed to download CRS, {last_failure}"
                ))
                .into());
            }
            Err(err) => {
                last_failure = err.to_string();
                if attempt < MAX_CRS_DOWNLOAD_ATTEMPTS {
                    let delay = exponential_backoff_delay(attempt);
                    warn!(
                        "CRS download request error, retrying in {:?} (attempt {attempt}/{MAX_CRS_DOWNLOAD_ATTEMPTS}): {err}",
                        delay
                    );
                    sleep(delay).await;
                    continue;
                }

                return Err(io::Error::other(format!(
                    "failed to download CRS: {err}"
                ))
                .into());
            }
        }
    }

    Err(io::Error::other(format!(
        "exceeded retry limit while downloading CRS, last failure: {last_failure}"
    ))
    .into())
}

fn should_retry_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS
        || status == StatusCode::REQUEST_TIMEOUT
        || status.is_server_error()
}
fn parse_retry_after_header(
    value: Option<&reqwest::header::HeaderValue>,
) -> Option<Duration> {
    let value = value?;
    let value = value.to_str().ok()?;
    let seconds = value.parse::<u64>().ok()?;
    Some(Duration::from_secs(seconds))
}

fn exponential_backoff_delay(attempt: usize) -> Duration {
    let exponent = (attempt.saturating_sub(1)).min(8) as u32;
    let multiplier = 1u64 << exponent;
    let delay_ms = BASE_RETRY_DELAY_MS.saturating_mul(multiplier);

    cmp::min(
        Duration::from_millis(delay_ms),
        Duration::from_secs(MAX_RETRY_DELAY_SECS),
    )
}

fn check_circuits_cache(
    circuit_list: Vec<CircuitProfile>,
) -> Result<(), io::Error> {
    let theme = Theme::default();
    for circuit in circuit_list {
        info!(
            "{} {} verifier data from cache",
            theme.action("Fetching"),
            circuit.name()
        );
        match circuit.get_verifier() {
            Ok(_) => {
                info!("{}   {}.vd", theme.info("Found"), circuit.id_str());
            }

            _ => {
                warn!("{} due to cache miss", theme.warn("Compiling"),);

                let compressed = circuit.get_compressed();
                let (pk, vd) = Compiler::compile_with_compressed(
                    &PUB_PARAMS,
                    TRANSCRIPT_LABEL,
                    compressed,
                )
                .map_err(|e| {
                    io::Error::other(format!(
                        "Couldn't compile keys for {}: {}",
                        circuit.name(),
                        e
                    ))
                })?;
                circuit.add_keys(pk.to_bytes(), vd.to_bytes())?;
                info!("{}   {}.vd", theme.info("Cached"), circuit.id_str());
                info!("{}   {}.pk", theme.info("Cached"), circuit.id_str());
            }
        }
    }
    Ok(())
}

fn circuits_from_names(
    names: &[&str],
) -> Result<Vec<CircuitProfile>, io::Error> {
    let mut circuits = Vec::new();
    for name in names {
        let circuit = CircuitProfile::from_name(name)?;
        circuits.push(circuit);
    }
    Ok(circuits)
}

fn run_stored_circuits_checks(
    keep_circuits: bool,
    circuit_list: Vec<CircuitProfile>,
) -> Result<(), io::Error> {
    let theme = Theme::default();

    if !keep_circuits {
        warn!("{} for untracked circuits", theme.warn("Checking"),);
        rusk_profile::clean_outdated(&circuit_list)?;
    } else {
        info!("{} untracked circuits", theme.action("Keeping"),);
    }
    check_circuits_cache(circuit_list).map(|_| ())
}

pub fn exec(
    keep_circuits: bool,
    crs_url: String,
) -> Result<(), Box<dyn std::error::Error>> {
    *CRS_URL.lock().expect("Unlocking failed.") = crs_url;

    // This force init is needed to check CRS and create it (if not available)
    // See also: https://github.com/dusk-network/rusk/issues/767
    LazyLock::force(&PUB_PARAMS);

    // cache all circuit descriptions, check if they changed
    circuits::cache_all()?;

    // create a list of the circuit names under whish they are stored
    // it is also possible to fetch a circuit by its ID, however that ID changes
    // when the circuit changes.
    let circuits = circuits_from_names(&[
        "TxCircuitOneTwo",
        "TxCircuitTwoTwo",
        "TxCircuitThreeTwo",
        "TxCircuitFourTwo",
    ])?;

    run_stored_circuits_checks(keep_circuits, circuits)?;

    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn test_crs() {
        LazyLock::force(&PUB_PARAMS);
    }
}
