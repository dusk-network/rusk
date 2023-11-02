// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Helper functions to get the proper version of the crate to be
//! displayed.

use std::sync::LazyLock;

#[inline]
pub(crate) fn show_version(verbose: bool) -> String {
    let info = rustc_tools_util::get_version_info!();
    let pre = std::env!("CARGO_PKG_VERSION_PRE");
    let version = if pre.is_empty() {
        format!("{}.{}.{}", info.major, info.minor, info.patch)
    } else {
        format!("{}.{}.{}-{}", info.major, info.minor, info.patch, pre)
    };
    let build = format!(
        "{} {}",
        info.commit_hash.unwrap_or_default(),
        info.commit_date.unwrap_or_default()
    );

    if verbose && build.trim().len() > 1 {
        format!("{version} ({build})")
    } else {
        version
    }
}

pub static VERSION_BUILD: LazyLock<String> =
    LazyLock::new(|| show_version(true));

pub static VERSION: LazyLock<String> = LazyLock::new(|| show_version(false));
