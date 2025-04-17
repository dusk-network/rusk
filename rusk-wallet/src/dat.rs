// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Methods for parsing/checking the DAT wallet file

use std::fs;
use std::io::Read;

use wallet_core::Seed;

use crate::crypto::{decrypt_aes_cbc, decrypt_aes_gcm};
use crate::{Error, WalletPath, IV_SIZE, SALT_SIZE};

/// Binary prefix for old Dusk wallet files
pub const OLD_MAGIC: u32 = 0x1d_0c15;
/// Binary prefix for new binary file format
pub const MAGIC: u32 = 0x72_736b;
/// The latest version of the rusk binary format for wallet dat file
pub const LATEST_VERSION: Version = (0, 0, 2, 0, false);
/// The type info of the dat file we'll save
pub const FILE_TYPE: u16 = 0x0200;
/// Reserved for futures use, 0 for now
pub const RESERVED: u16 = 0x0000;
/// (Major, Minor, Patch, Pre, Pre-Higher)
type Version = (u8, u8, u8, u8, bool);

type Salt = [u8; SALT_SIZE];
type Iv = [u8; IV_SIZE];

const FILE_HEADER_SIZE: usize = 12;

/// Versions of the potential wallet DAT files we read
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FileVersion {
    /// Legacy the oldest format
    Legacy,
    /// Preciding legacy, we have the old one
    OldWalletCli(Version),
    /// The newest one. All new saves are saved in this file format
    RuskBinaryFileFormat(Version),
}

impl FileVersion {
    /// Checks if the file version is older than the latest Rust Binary file
    /// format
    #[must_use]
    pub fn is_old(&self) -> bool {
        match self {
            Self::Legacy | Self::OldWalletCli(_) => true,
            Self::RuskBinaryFileFormat(version) => version < &LATEST_VERSION,
        }
    }
}

fn read_salt_and_iv(
    version: FileVersion,
    bytes: &[u8],
) -> Result<Option<(Salt, Iv)>, Error> {
    match version {
        FileVersion::Legacy | FileVersion::OldWalletCli(_) => Ok(None),
        FileVersion::RuskBinaryFileFormat(version)
            if version_without_pre_higher(version) < (0, 0, 2, 0) =>
        {
            Ok(None)
        }
        FileVersion::RuskBinaryFileFormat(_) => {
            if let (Some(salt_bytes), Some(iv_bytes)) = (
                bytes.get(FILE_HEADER_SIZE..FILE_HEADER_SIZE + SALT_SIZE),
                bytes.get(
                    FILE_HEADER_SIZE + SALT_SIZE
                        ..FILE_HEADER_SIZE + SALT_SIZE + IV_SIZE,
                ),
            ) {
                let salt = salt_bytes
                    .try_into()
                    .map_err(|_| Error::WalletFileCorrupted)?;
                let iv = iv_bytes
                    .try_into()
                    .map_err(|_| Error::WalletFileCorrupted)?;
                Ok(Some((salt, iv)))
            } else {
                Err(Error::WalletFileCorrupted)
            }
        }
    }
}

/// Make sense of the payload and return it
pub(crate) fn get_seed_and_address(
    file: FileVersion,
    mut bytes: Vec<u8>,
    aes_key: &[u8],
    iv: Option<&[u8; IV_SIZE]>,
) -> Result<(Seed, u8), Error> {
    match file {
        FileVersion::Legacy => {
            if bytes[1] == 0 && bytes[2] == 0 {
                bytes.drain(..3);
            }

            bytes = decrypt_aes_cbc(&bytes, aes_key)?;

            // get our seed
            let seed = bytes[..]
                .try_into()
                .map_err(|_| Error::WalletFileCorrupted)?;

            Ok((seed, 1))
        }
        FileVersion::OldWalletCli((major, minor, _, _, _)) => {
            bytes.drain(..5);

            let result: Result<(Seed, u8), Error> = match (major, minor) {
                (1, 0) => {
                    let content = decrypt_aes_cbc(&bytes, aes_key)?;
                    let buff = &content[..];

                    let seed = buff
                        .try_into()
                        .map_err(|_| Error::WalletFileCorrupted)?;

                    Ok((seed, 1))
                }
                (2, 0) => {
                    let content = decrypt_aes_cbc(&bytes, aes_key)?;
                    let buff = &content[..];

                    // extract seed
                    let seed = buff
                        .try_into()
                        .map_err(|_| Error::WalletFileCorrupted)?;

                    // extract addresses count
                    Ok((seed, buff[0]))
                }
                _ => Err(Error::UnknownFileVersion(major, minor)),
            };

            result
        }
        FileVersion::RuskBinaryFileFormat(version) => {
            const OLD_PAYLOAD_SIZE: usize = 96;
            const PAYLOAD_SIZE: usize = 81;

            let (rest, use_aes_gcm) =
                if version_without_pre_higher(version) < (0, 0, 2, 0) {
                    let offset = FILE_HEADER_SIZE;
                    (bytes.get(offset..(offset + OLD_PAYLOAD_SIZE)), false)
                } else {
                    let offset = FILE_HEADER_SIZE + SALT_SIZE + IV_SIZE;
                    (bytes.get(offset..(offset + PAYLOAD_SIZE)), true)
                };

            if let Some(rest) = rest {
                let content = if use_aes_gcm {
                    let iv = iv.ok_or(Error::WalletFileCorrupted)?;
                    decrypt_aes_gcm(rest, aes_key, iv)?
                } else {
                    decrypt_aes_cbc(rest, aes_key)?
                };

                if let Some(seed_buff) = content.get(0..65) {
                    let seed = seed_buff[0..64]
                        .try_into()
                        .map_err(|_| Error::WalletFileCorrupted)?;

                    let count = &seed_buff[64..65];

                    Ok((seed, count[0]))
                } else {
                    Err(Error::WalletFileCorrupted)
                }
            } else {
                Err(Error::WalletFileCorrupted)
            }
        }
    }
}

/// From the first 12 bytes of the file [header], we check version
///
/// [header]: https://github.com/dusk-network/rusk/wiki/Binary-File-Format/#header
pub(crate) fn check_version(
    bytes: Option<&[u8]>,
) -> Result<FileVersion, Error> {
    match bytes {
        Some(bytes) => {
            let header_bytes: [u8; 4] = bytes[0..4]
                .try_into()
                .map_err(|_| Error::WalletFileCorrupted)?;

            let magic = u32::from_le_bytes(header_bytes) & 0x00ff_ffff;

            if magic == OLD_MAGIC {
                // check for version information
                let (major, minor) = (bytes[3], bytes[4]);

                Ok(FileVersion::OldWalletCli((major, minor, 0, 0, false)))
            } else {
                let header_bytes = bytes[0..8]
                    .try_into()
                    .map_err(|_| Error::WalletFileCorrupted)?;

                let number = u64::from_be_bytes(header_bytes);

                let magic_num = (number & 0xff_ffff_0000_0000) >> 32;

                if (magic_num as u32) != MAGIC {
                    return Ok(FileVersion::Legacy);
                }

                let file_type = (number & 0x00_0000_ffff_0000) >> 16;
                let reserved = number & 0x00_0000_0000_ffff;

                if file_type != u64::from(FILE_TYPE) {
                    return Err(Error::WalletFileCorrupted);
                };

                if reserved != u64::from(RESERVED) {
                    return Err(Error::WalletFileCorrupted);
                };

                let version_bytes = bytes[8..12]
                    .try_into()
                    .map_err(|_| Error::WalletFileCorrupted)?;

                let version = u32::from_be_bytes(version_bytes);

                let major = (version & 0xff00_0000) >> 24;
                let minor = (version & 0x00ff_0000) >> 16;
                let patch = (version & 0x0000_ff00) >> 8;
                let pre = (version & 0x0000_00f0) >> 4;
                let higher = version & 0x000_0000f;

                let pre_higher = matches!(higher, 1);

                Ok(FileVersion::RuskBinaryFileFormat((
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
///
/// # Errors
/// This function will error if the file is missing or invalid.
pub fn read_file_version(file: &WalletPath) -> Result<FileVersion, Error> {
    let path = &file.wallet;

    // make sure file exists
    if !path.is_file() {
        return Err(Error::WalletFileMissing);
    }

    let mut fs = fs::File::open(path)?;

    let mut header_buf = [0; 12];

    fs.read_exact(&mut header_buf)?;

    check_version(Some(&header_buf))
}

/// Read the file version of the dat file from the header and, if present,
/// the salt and IV.
///
/// # Errors
/// This function will error if the wallet-file is corrupted.
pub fn read_file_version_and_salt_iv(
    file: &WalletPath,
) -> Result<(FileVersion, Option<(Salt, Iv)>), Error> {
    let path = &file.wallet;

    if !path.is_file() {
        return Err(Error::WalletFileMissing);
    }

    let mut fs = fs::File::open(path)?;
    let mut buf = [0; FILE_HEADER_SIZE + SALT_SIZE + IV_SIZE];
    fs.read_exact(&mut buf)?;
    let version = check_version(Some(&buf[..FILE_HEADER_SIZE]))?;
    let salt = read_salt_and_iv(version, &buf)?;

    Ok((version, salt))
}

pub(crate) fn version_bytes(version: Version) -> [u8; 4] {
    u32::from_be_bytes([version.0, version.1, version.2, version.3])
        .to_be_bytes()
}

/// Returns the given version with its last item, pre-higher, removed
#[must_use]
pub fn version_without_pre_higher(version: Version) -> (u8, u8, u8, u8) {
    (version.0, version.1, version.2, version.3)
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
        // rusk binary headers
        let rusk_bin_file_1 = vec![
            0x00, 0x72, 0x73, 0x6b, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
            0x00,
        ];
        let rusk_bin_file_2 = vec![
            0x00, 0x72, 0x73, 0x6b, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02,
            0x00,
        ];

        assert_eq!(
            check_version(Some(&old_wallet_file)).unwrap(),
            FileVersion::OldWalletCli((2, 0, 0, 0, false))
        );

        assert_eq!(
            check_version(Some(&legacy_file)).unwrap(),
            FileVersion::Legacy
        );

        assert_eq!(
            check_version(Some(&rusk_bin_file_1)).unwrap(),
            FileVersion::RuskBinaryFileFormat((0, 0, 1, 0, false))
        );

        assert_eq!(
            check_version(Some(&rusk_bin_file_2)).unwrap(),
            FileVersion::RuskBinaryFileFormat((0, 0, 2, 0, false))
        );
    }
}
