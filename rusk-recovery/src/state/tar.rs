// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::error::Error;
use std::path::Path;

use super::zip;

/// Unarchive files into a destination folder
pub fn unarchive(buffer: &[u8], output: &Path) -> Result<(), Box<dyn Error>> {
    zip::unzip(buffer, output)
}

/// Archive a folder into a destination file.
pub fn archive(src_dir: &Path, dst_file: &Path) -> Result<(), Box<dyn Error>> {
    Ok(())
}
