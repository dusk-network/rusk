// Copyright (c) DUSK NETWORK. All rights reserved.
// Licensed under the MPL 2.0 license. See LICENSE file in the project root for details.

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

    let version_build = if build.len() > 1 {
        format!("{} ({})", version, build)
    } else {
        version
    };

    version_build
}
