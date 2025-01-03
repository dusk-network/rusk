// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

/// RUES binary format constants
pub mod binary {
    /// RUES magic number ('0rsk')
    pub const MAGIC_NUMBER: u32 = 0x0072_736b;

    /// File types
    pub mod file_type {
        /// Rusk Smart Contract
        pub const SMART_CONTRACT: u16 = 0x01;
        /// Rusk Wallet
        pub const WALLET: u16 = 0x02;
    }

    /// Field sizes in bytes
    pub mod size {
        /// Magic number field size
        pub const MAGIC: usize = 4;
        /// File type field size
        pub const FILE_TYPE: usize = 2;
        /// Reserved field size
        pub const RESERVED: usize = 2;
        /// Version field size
        pub const VERSION: usize = 4;
        /// Full header size
        pub const HEADER: usize = MAGIC + FILE_TYPE + RESERVED + VERSION;
    }

    /// Field offsets in bytes
    pub mod offset {
        /// File type field offset
        pub const FILE_TYPE: usize = super::size::MAGIC;
        /// Reserved field offset
        pub const RESERVED: usize = FILE_TYPE + super::size::FILE_TYPE;
        /// Version field offset
        pub const VERSION: usize = RESERVED + super::size::RESERVED;
    }
}
