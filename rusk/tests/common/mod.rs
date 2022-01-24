// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod encoding;
#[cfg(not(target_os = "windows"))]
pub mod unix;

use futures::future::BoxFuture;
use std::convert::TryFrom;
use std::io;
use std::path::PathBuf;
use std::task::{Context, Poll};
use tempfile::tempdir;

use canonical::{Canon, Sink, Source};
use dusk_abi::ContractState;
use futures::TryFutureExt;
use microkelvin::{Backend, BackendCtor, DiskBackend};
use rusk::{Result, Rusk};
use stake_contract::StakeContract;
use tokio::net::{UnixListener, UnixStream};
use tonic::transport::{Channel, Endpoint, Uri};
use tower::Service;
use tracing::info;
use tracing_subscriber::EnvFilter;
use transfer_contract::TransferContract;

/// This function creates a temporary backend for testing purposes.
/// Each function creates its own backend, so to avoid side effects tests that
/// are modifying the state should define their own backend.
/// This can be used for tests that does not modify the state, or needs to
/// read the default state.
pub fn testbackend() -> BackendCtor<DiskBackend> {
    BackendCtor::new(DiskBackend::ephemeral)
}

pub fn update_transfer_contract<B>(
    rusk: &mut Rusk,
    transfer: TransferContract,
    ctor: &BackendCtor<B>,
) -> Result<()>
where
    B: 'static + Backend,
{
    let mut rusk_state = rusk.state()?;

    const PAGE_SIZE: usize = 1024 * 64;
    let mut bytes = [0u8; PAGE_SIZE];
    let mut sink = Sink::new(&mut bytes[..]);
    ContractState::from_canon(&transfer).encode(&mut sink);
    let mut source = Source::new(&bytes[..]);
    let contract_state = ContractState::decode(&mut source)?;
    *rusk_state
        .inner_mut()
        .get_contract_mut(&rusk_abi::transfer_contract())?
        .state_mut() = contract_state;
    let state_id = rusk_state.persist(ctor)?;

    *rusk.state_id.lock() = state_id;

    Ok(())
}

pub fn update_stake_contract<B>(
    rusk: &mut Rusk,
    stake: StakeContract,
    ctor: &BackendCtor<B>,
) -> Result<()>
where
    B: 'static + Backend,
{
    let mut rusk_state = rusk.state()?;

    const PAGE_SIZE: usize = 1024 * 64;
    let mut bytes = [0u8; PAGE_SIZE];
    let mut sink = Sink::new(&mut bytes[..]);
    ContractState::from_canon(&stake).encode(&mut sink);
    let mut source = Source::new(&bytes[..]);
    let contract_state = ContractState::decode(&mut source)?;
    *rusk_state
        .inner_mut()
        .get_contract_mut(&rusk_abi::stake_contract())?
        .state_mut() = contract_state;
    let state_id = rusk_state.persist(ctor)?;

    *rusk.state_id.lock() = state_id;

    Ok(())
}

pub fn logger() {
    // Can't use `with_default_env` since we want to have a default
    // directive, and *then* apply the environment variable on top of it,
    // not the other way around.
    let directive = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "rusk=info,tests=info".to_string());
    let filter = EnvFilter::new(directive);

    let _ = tracing_subscriber::fmt().with_env_filter(filter).try_init();
}

// pub async fn setup<B>(ctor: &BackendCtor<B>) -> (Channel, Rusk)
// where
//     B: 'static + Backend,
pub async fn setup() -> (
    Channel,
    async_stream::AsyncStream<
        Result<unix::UnixStream, std::io::Error>,
        impl futures::Future<Output = ()>,
    >,
) where {
    logger();
    // Creates a temporary file for the socket
    let tempdir = tempdir().expect("failed to create tmp");
    let socket_path = tempdir.path().join("socket");

    info!("creating socket at {:?}", socket_path);

    let uds =
        UnixListener::bind(&socket_path).expect("Error binding the socket");

    let incoming = async_stream::stream! {
        loop {
            yield uds.accept().map_ok(|(st, _)| unix::UnixStream(st)).await
        }
    };
    // Create the client bound to the default testing UDS path.
    let channel = Endpoint::try_from("http://[::]:50051")
        .expect("Serde error on addr reading")
        .connect_with_connector(UdsConnector::from(socket_path.clone()))
        .await
        .expect("Error generating a Channel");

    (channel, incoming)
}

/// A connector to a UDS with a particular path.
struct UdsConnector {
    path: PathBuf,
}

impl From<PathBuf> for UdsConnector {
    fn from(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Service<Uri> for UdsConnector {
    type Response = UnixStream;
    type Error = io::Error;
    type Future = BoxFuture<'static, io::Result<UnixStream>>;

    fn poll_ready(
        &mut self,
        _: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, _: Uri) -> Self::Future {
        Box::pin(UnixStream::connect(self.path.clone()))
    }
}
