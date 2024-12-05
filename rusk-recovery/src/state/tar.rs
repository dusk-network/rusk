// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::error::Error;
use std::fs::File;
use std::path::Path;

use flate2::{read, write, Compression};
use tar::Archive;

use super::zip;

/// Unarchive files into a destination folder
pub fn unarchive(buffer: &[u8], output: &Path) -> Result<(), Box<dyn Error>> {
    let tar = read::GzDecoder::new(buffer);
    let mut archive = Archive::new(tar);
    archive
        .unpack(output)
        .or_else(|_| zip::unzip(buffer, output))
}

/// Archive a folder into a destination file.
pub fn archive(src_dir: &Path, dst_file: &Path) -> Result<(), Box<dyn Error>> {
    let tar_gz = File::create(dst_file)?;
    let enc = write::GzEncoder::new(tar_gz, Compression::default());
    let mut tar = tar::Builder::new(enc);
    tar.append_dir_all("", src_dir)?;
    Ok(())
}
