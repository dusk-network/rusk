// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Methods for parsing/checking the DAT wallet file

use std::fs;
use std::io::Read;

use crate::{Error, WalletFilePath, WalletPath};

/// Binary prefix for old Dusk wallet files
pub const OLD_MAGIC: u32 = 0x1d0c15;
/// Binary prefix for new binary file format
pub const MAGIC: u32 = 0x72736b;
/// The latest version of the rusk binary format for wallet dat file
pub const LATEST_VERSION: Version = (0, 0, 1, 0, false);
/// The type info of the dat file we'll save
pub const FILE_TYPE: u16 = 0x0200;
/// Reserved for futures use, 0 for now
pub const RESERVED: u16 = 0x0000;
/// (Major, Minor, Patch, Pre, Pre-Higher)
type Version = (u8, u8, u8, u8, bool);

/// Versions of the potential wallet DAT files we read
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DatFileVersion {
    /// Legacy the oldest format
    Legacy,
    /// Preciding legacy, we have the old one
    OldWalletCli(Version),
    /// The newest one. All new saves are saved in this file format
    RuskBinaryFileFormat(Version),
}

/// From the first 12 bytes of the file (header), we check version
///
/// https://github.com/dusk-network/rusk/wiki/Binary-File-Format/#header
pub(crate) fn check_version(
    bytes: Option<&[u8]>,
) -> Result<DatFileVersion, Error> {
    match bytes {
        Some(bytes) => {
            let header_bytes: [u8; 4] = bytes[0..4]
                .try_into()
                .map_err(|_| Error::WalletFileCorrupted)?;

            let magic = u32::from_le_bytes(header_bytes) & 0x00ffffff;

            if magic == OLD_MAGIC {
                // check for version information
                let (major, minor) = (bytes[3], bytes[4]);

                Ok(DatFileVersion::OldWalletCli((major, minor, 0, 0, false)))
            } else {
                let header_bytes = bytes[0..8]
                    .try_into()
                    .map_err(|_| Error::WalletFileCorrupted)?;

                let number = u64::from_be_bytes(header_bytes);

                let magic_num = (number & 0xFFFFFF00000000) >> 32;

                if (magic_num as u32) != MAGIC {
                    return Ok(DatFileVersion::Legacy);
                }

                let file_type = (number & 0x000000FFFF0000) >> 16;
                let reserved = number & 0x0000000000FFFF;

                if file_type != FILE_TYPE as u64 {
                    return Err(Error::WalletFileCorrupted);
                };

                if reserved != RESERVED as u64 {
                    return Err(Error::WalletFileCorrupted);
                };

                let version_bytes = bytes[8..12]
                    .try_into()
                    .map_err(|_| Error::WalletFileCorrupted)?;

                let version = u32::from_be_bytes(version_bytes);

                let major = (version & 0xff000000) >> 24;
                let minor = (version & 0x00ff0000) >> 16;
                let patch = (version & 0x0000ff00) >> 8;
                let pre = (version & 0x000000f0) >> 4;
                let higher = version & 0x0000000f;

                let pre_higher = matches!(higher, 1);

                Ok(DatFileVersion::RuskBinaryFileFormat((
                    major as u8,
                    minor as u8,
                    patch as u8,
                    pre as u8,
                    pre_higher,
                )))
            }
        }
        None => Err(Error::WalletFileCorrupted),
    }
}

/// Read the first 12 bytes of the dat file and get the file version from
/// there
pub fn read_file_version(
    wallet_file_path: &WalletPath,
) -> Result<DatFileVersion, Error> {
    let path = &wallet_file_path.wallet_path();

    // make sure file exists
    if !path.is_file() {
        return Err(Error::WalletFileMissing);
    }

    let mut fs = fs::File::open(path)?;

    let mut header_buf = [0; 12];

    fs.read_exact(&mut header_buf)?;

    check_version(Some(&header_buf))
}

pub(crate) fn version_bytes(version: Version) -> [u8; 4] {
    u32::from_be_bytes([version.0, version.1, version.2, version.3])
        .to_be_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distiction_between_versions() {
        // with magic number
        let old_wallet_file = vec![0x15, 0x0c, 0x1d, 0x02, 0x00];
        // no magic number just nonsense bytes
        let legacy_file = vec![
            0xab, 0x38, 0x81, 0x3b, 0xfc, 0x79, 0x11, 0xf9, 0x86, 0xd6, 0xd0,
        ];
        // new header
        let new_file = vec![
            0x00, 0x72, 0x73, 0x6b, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
            0x00,
        ];

        assert_eq!(
            check_version(Some(&old_wallet_file)).unwrap(),
            DatFileVersion::OldWalletCli((2, 0, 0, 0, false))
        );

        assert_eq!(
            check_version(Some(&legacy_file)).unwrap(),
            DatFileVersion::Legacy
        );

        assert_eq!(
            check_version(Some(&new_file)).unwrap(),
            DatFileVersion::RuskBinaryFileFormat((0, 0, 1, 0, false))
        );
    }
}
