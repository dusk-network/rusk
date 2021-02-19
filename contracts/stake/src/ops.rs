// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

// Queries
/// Opcode for finding a specific stake in the contract.
pub const QR_FIND_STAKE: u16 = 0x00;

// Transactions
/// Opcode for adding a stake to the contract.
pub const TX_STAKE: u16 = 0x01;
/// Opcode for extending an existing stake in the contract.
pub const TX_EXTEND_STAKE: u16 = 0x02;
/// Opcode for retrieving an existing stake in the contract.
pub const TX_WITHDRAW_STAKE: u16 = 0x03;
/// Opcode for punishing a malicious provisioner.
pub const TX_SLASH: u16 = 0x04;
