// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

mod args;
mod gql;

pub(crate) mod prompt;
pub(crate) mod status;

pub(crate) use args::WalletArgs;
pub(crate) use gql::GraphQL;
