// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::error::Error;
use std::fs;
use std::io::Cursor;
use std::path::Path;

use zip::ZipArchive;

/// Unzip binaries into a destination folder
pub fn unzip(buffer: &[u8], output: &Path) -> Result<(), Box<dyn Error>> {
    let reader = Cursor::new(buffer);
    let mut zip = ZipArchive::new(reader)?;
    fs::create_dir_all(output)?;
    zip.extract(output)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::tempdir;
    use zip::write::SimpleFileOptions;
    use zip::{CompressionMethod, ZipWriter};

    use super::*;

    #[test]
    fn unzip_enforces_entry_path_safety() {
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored);
        let cases = [
            ("../escape.txt", false),
            ("/escape.txt", false),
            ("safe/dir/file.txt", true),
        ];

        for (entry_path, should_succeed) in cases {
            let mut cursor = Cursor::new(Vec::new());
            let mut writer = ZipWriter::new(&mut cursor);
            writer.start_file(entry_path, options).unwrap();
            writer.write_all(b"ok").unwrap();
            writer.finish().unwrap();

            let archive = cursor.into_inner();
            let out = tempdir().unwrap();
            let result = unzip(&archive, out.path());

            if should_succeed {
                assert!(
                    result.is_ok(),
                    "expected successful unzip for entry path {entry_path}"
                );
                assert_eq!(
                    fs::read(out.path().join("safe/dir/file.txt")).unwrap(),
                    b"ok"
                );
            } else {
                assert!(
                    result.is_err(),
                    "expected extraction failure for {entry_path}"
                );
            }
        }
    }
}
