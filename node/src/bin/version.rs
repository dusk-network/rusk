// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Helper functions to get the proper version of the crate to be
//! displayed.

use rustc_tools_util::VersionInfo;

#[inline]
pub(crate) fn show_version(info: VersionInfo) -> String {
    let version = format!("{}.{}.{}", info.major, info.minor, info.patch);
    let build = format!(
        "{} {}",
        info.commit_hash.unwrap_or_default(),
        info.commit_date.unwrap_or_default()
    );

    if build.len() > 1 {
        format!("{} ({})", version, build)
    } else {
        version
    }
}
