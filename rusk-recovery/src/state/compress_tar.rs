// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

#[cfg(feature = "gz")]
use flate2::{
    read::GzDecoder as Uncompressor, write::GzEncoder as Compressor,
    Compression,
};
#[cfg(feature = "xz")]
use xz2::{read::XzDecoder as Uncompressor, write::XzEncoder as Compressor};

use std::error::Error;
use std::fs::File;
use std::io::{self, ErrorKind};
use std::path::Path;
use tar::Archive;

pub(crate) fn uncompress(
    data: &[u8],
    target_dir: &Path,
) -> Result<(), Box<dyn Error>> {
    Archive::new(Uncompressor::new(data))
        .unpack(target_dir)
        .map_err(|err| err.into())
}

// QUESTION: create Tar archive manually, with output of processed files?
/// Compress a folder into a destination file.
pub fn compress(
    src_path: &Path,
    dst_file: &Path,
) -> Result<(), Box<dyn Error>> {
    if !src_path.is_dir() {
        return Err(io::Error::from(ErrorKind::NotFound).into());
    }
    let archive_file = File::create(&dst_file)?;
    #[cfg(feature = "gz")]
    let encoder = Compressor::new(archive_file, Compression::default());
    #[cfg(feature = "xz")]
    let encoder = Compressor::new(archive_file, 6);
    let mut tar = tar::Builder::new(encoder);
    tar.append_dir_all("", src_path)?;
    tar.finish().map_err(|err| err.into())
}
