// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};

/// Represents the activation condition for a feature or host query.
///
/// This enum defines when a specific feature or host query becomes active
/// based on block heights. It can either be activated at a specific height
/// or within specified ranges of block heights.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Activation {
    /// Activation at a specific block height.
    Height(u64),
    /// Activation within specified ranges (including the lower and upper
    /// bound) of block heights.
    Ranges(Vec<(u64, u64)>),
}

impl Activation {
    /// Checks if the feature is active at the given block height.
    pub fn is_active_at(&self, height: u64) -> bool {
        match self {
            Activation::Height(activation_height) => {
                height >= *activation_height
            }
            Activation::Ranges(ranges) => ranges
                .iter()
                .any(|(start, end)| height >= *start && height <= *end),
        }
    }

    /// Unwraps the activation height.
    ///
    /// Panics if the activation is of type `Activation::Ranges`.
    pub fn unwrap_height(&self) -> u64 {
        match self {
            Activation::Height(height) => *height,
            Activation::Ranges(_) => {
                panic!("Called unwrap_height on Activation::Ranges")
            }
        }
    }

    /// Unwraps the activation ranges.
    ///
    /// Panics if the activation is of type `Activation::Height`.
    pub fn unwrap_ranges(&self) -> &[(u64, u64)] {
        match self {
            Activation::Height(_) => {
                panic!("Called unwrap_height on Activation::Height")
            }
            Activation::Ranges(ranges) => &ranges[..],
        }
    }
}

impl From<u64> for Activation {
    fn from(height: u64) -> Self {
        Activation::Height(height)
    }
}

impl From<Vec<(u64, u64)>> for Activation {
    fn from(ranges: Vec<(u64, u64)>) -> Self {
        Activation::Ranges(ranges)
    }
}

impl Display for Activation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Activation::Height(height) => {
                write!(f, "Height({})", height)
            }
            Activation::Ranges(ranges) => {
                let ranges_str = ranges
                    .iter()
                    .map(|(start, end)| format!("({start},{end})"))
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "Ranges([{ranges_str}])")
            }
        }
    }
}
