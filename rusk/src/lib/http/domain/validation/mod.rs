// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

pub mod context;
pub mod rules;

use crate::http::domain::error::DomainError;

/// Core validation trait
#[async_trait::async_trait]
pub trait Validator<T>: Send + Sync {
    async fn validate(
        &self,
        value: &T,
        ctx: &mut context::ValidationContext,
    ) -> Result<(), DomainError>;
}
