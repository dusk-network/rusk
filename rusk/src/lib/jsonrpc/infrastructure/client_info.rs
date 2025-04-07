// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Defines the `ClientInfo` struct used for identifying clients, primarily for
//! rate limiting purposes.

use std::fmt;
use std::net::SocketAddr;

/// Information about a client making a request.
///
/// Currently just wraps a socket address, but may be extended in the future
/// to include more client identification data (e.g., auth tokens, etc.).
///
/// Implements `Clone`, `Hash`, `Eq`, `PartialEq` to be used as a key in
/// collections like `DashMap` and for `governor`.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ClientInfo(SocketAddr);

impl ClientInfo {
    /// Create a new ClientInfo from a socket address.
    pub fn new(addr: SocketAddr) -> Self {
        Self(addr)
    }

    /// Returns the underlying socket address.
    pub fn socket_addr(&self) -> SocketAddr {
        self.0
    }

    /// Create a new ClientInfo from IP address and port
    pub fn from_ip(ip: std::net::IpAddr, port: u16) -> Self {
        Self(SocketAddr::new(ip, port))
    }
}

impl fmt::Display for ClientInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
