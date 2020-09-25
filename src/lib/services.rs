// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.

pub mod echoer;
pub mod pki;
pub mod staking;
use tonic::{Request, Response, Status};

pub(crate) mod rusk_proto {
    tonic::include_proto!("rusk");
}

/// A trait that defines the general workflow that the handlers for every
/// GRPC request should follow.
/// The trait is designed to just hold a reference to the request all of the
/// time so that there's no memory waste.
pub trait ServiceRequestHandler<'a, 'b, T, U> {
    /// Generates a Handler with a reference to the request stored
    /// inside of it.
    fn load_request(request: &'b Request<T>) -> Self;
    /// Handles a Request
    fn handle_request(&self) -> Result<Response<U>, Status>;
}
