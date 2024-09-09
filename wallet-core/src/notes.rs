// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Provides functions and types for interacting with notes.

/// Module for balance information.
pub mod balance;
/// Module for owned notes.
pub mod owned;
/// Module for picking notes.
pub mod pick;

/// The maximum amount of input notes that can be spend in one
/// phoenix-transaction
pub const MAX_INPUT_NOTES: usize = 4;
