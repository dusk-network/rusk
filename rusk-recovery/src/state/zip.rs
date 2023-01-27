// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::error::Error;
use std::fs::{self};
use std::io::{Cursor, Read};
use std::path::Path;

use zip::ZipArchive;

/// Unzip binaries into a destination folder
pub fn unzip(buffer: &[u8], output: &Path) -> Result<(), Box<dyn Error>> {
    let reader = Cursor::new(buffer);
    let mut zip = ZipArchive::new(reader)?;

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let entry_path = output.join(entry.name());

        if entry.is_dir() {
            fs::create_dir_all(entry_path)?;
        } else {
            let mut buffer = Vec::with_capacity(entry.size() as usize);
            entry.read_to_end(&mut buffer)?;
            fs::write(entry_path, buffer)?;
        }
    }
    Ok(())
}
