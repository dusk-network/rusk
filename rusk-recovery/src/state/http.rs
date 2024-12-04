// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::error::Error;

use http_req::request;

const MAX_REDIRECT: usize = 3;

pub(super) fn download<T>(uri: T) -> Result<Vec<u8>, Box<dyn Error>>
where
    T: AsRef<str>,
{
    download_with_redirect(uri, MAX_REDIRECT)
}

fn download_with_redirect<T>(
    uri: T,
    redirect_left: usize,
) -> Result<Vec<u8>, Box<dyn Error>>
where
    T: AsRef<str>,
{
    let mut buffer = vec![];

    let response = request::get(uri, &mut buffer)?;
    let sc = response.status_code();
    if sc.is_success() {
        return Ok(buffer);
    }
    if sc.is_redirect() && redirect_left > 1 {
        if let Some(uri) = response.headers().get("location") {
            return download_with_redirect(uri, redirect_left - 1);
        }
    }

    Err(format!("State download error: {response:?}").into())
}
