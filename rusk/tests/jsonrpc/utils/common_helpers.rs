// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, SanType};
use rusk::jsonrpc::config::{ConfigError, HttpServerConfig};
use tempfile::{tempdir, TempDir};

use std::fs;
use std::net::{IpAddr, Ipv4Addr};

/// Helper to get an ephemeral port by binding to port 0.
pub fn get_ephemeral_port() -> Result<std::net::SocketAddr, std::io::Error> {
    // Bind to port 0 to get an OS-assigned ephemeral port
    let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    // Drop the listener immediately to free the port for the actual server
    drop(listener);
    Ok(addr)
}

pub fn assert_security_error<T>(
    result: &Result<T, ConfigError>,
    expected_substring: &str,
) {
    if let Err(e) = result {
        let error_string_lower = e.to_string().to_lowercase();
        let expected_substring_lower = expected_substring.to_lowercase();
        assert!(
            error_string_lower.contains(&expected_substring_lower),
            "Expected error message to contain (case-insensitive) '{}', but got: {}",
            expected_substring,
            e
        );
    } else {
        panic!(
            "Expected an error containing '{}', but got Ok",
            expected_substring
        );
    }
}

pub fn generate_tls_certs(
) -> Result<(TempDir, HttpServerConfig), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let cert_path = dir.path().join("cert.pem");
    let key_path = dir.path().join("key.pem");

    let mut params = CertificateParams::new(vec!["localhost".to_string()])?;
    params.distinguished_name = DistinguishedName::new();
    params
        .distinguished_name
        .push(DnType::CommonName, "Rusk Test Cert");
    params
        .subject_alt_names
        .push(SanType::IpAddress(IpAddr::V4(Ipv4Addr::LOCALHOST)));

    let key_pair = KeyPair::generate()?;
    let cert = params.self_signed(&key_pair)?;

    // Use correct rcgen 0.13 methods with 'pem' feature
    let cert_pem = cert.pem(); // Get cert PEM string
    let key_pem = key_pair.serialize_pem(); // Serialize keypair to PEM string

    fs::write(&cert_path, cert_pem)?;
    fs::write(&key_path, key_pem)?;

    let http_config = HttpServerConfig {
        // Use a fixed, likely available port for testing instead of 0
        // If this port is taken, the test will fail, indicating need for a
        // different approach
        bind_address: "127.0.0.1:39989".parse()?,
        cert: Some(cert_path),
        key: Some(key_path),
        ..Default::default()
    };

    Ok((dir, http_config))
}
