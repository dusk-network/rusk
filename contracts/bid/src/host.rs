// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Host interface for the Bid Contract.
//!
//! Here the interface of the contract that will be used to execute
//! functions of it from the host envoirnoment (Rust) is defined here.
//!
//! It mostly contains the function signatures that need to be exported
//! to the outside world (AKA outside WASM).

mod transaction;
