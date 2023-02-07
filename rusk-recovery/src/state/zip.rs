// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fs::{self, File};
use std::io::{Cursor, Read, Seek, Write};
use std::path::Path;
use std::{error::Error, path::PathBuf};

use tracing::log::info;
use walkdir::{DirEntry, WalkDir};
use zip::result::ZipError;
use zip::write::FileOptions;
use zip::ZipArchive;

use crate::theme::Theme;

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

/// Zip a folder into a destination file.
pub fn zip(src_dir: &Path, dst_file: &Path) -> Result<(), Box<dyn Error>> {
    if !Path::new(src_dir).is_dir() {
        Err(ZipError::FileNotFound)?;
    }

    let path = Path::new(dst_file);
    let file = File::create(path)?;

    let walkdir = WalkDir::new(src_dir);
    let it = walkdir.into_iter();

    zip_dir(&mut it.filter_map(|e| e.ok()), &src_dir.to_path_buf(), file)?;

    Ok(())
}

fn zip_dir<T>(
    it: &mut dyn Iterator<Item = DirEntry>,
    prefix: &PathBuf,
    writer: T,
) -> Result<(), Box<dyn Error>>
where
    T: Write + Seek,
{
    let theme = Theme::default();
    let mut zip = zip::ZipWriter::new(writer);
    let options = FileOptions::default().unix_permissions(0o755);

    let mut buffer = Vec::new();
    for entry in it {
        let path = entry.path();
        let name = path.strip_prefix(Path::new(prefix))?;

        // Write file or directory explicitly
        // Some unzip tools unzip files with directory paths correctly, some do
        // not!
        if path.is_file() {
            info!("{} {:?} as {:?}", theme.info("Zipping"), path, name);
            #[allow(deprecated)]
            zip.start_file_from_path(name, options)?;
            let mut f = File::open(path)?;

            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
            buffer.clear();
        } else if !name.as_os_str().is_empty() {
            // Only if not root! Avoids path spec / warning
            // and mapname conversion failed error on unzip
            info!("{} dir {:?} as {:?}", theme.info("Zipping"), path, name);
            #[allow(deprecated)]
            zip.add_directory_from_path(name, options)?;
        }
    }
    zip.finish()?;
    Result::Ok(())
}
