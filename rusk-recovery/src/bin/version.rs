// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use rustc_tools_util::*;

#[inline]
pub fn version(info: &VersionInfo) -> String {
    let version = format!("{}.{}.{}", info.major, info.minor, info.patch);
    let build = format!(
        "{} {}",
        info.commit_hash.as_ref().unwrap_or(&"".to_string()),
        info.commit_date.as_ref().unwrap_or(&"".to_string())
    );

    if build.len() > 1 {
        format!("{} ({})", version, build)
    } else {
        version
    }
}
