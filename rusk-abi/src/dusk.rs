// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Dusk denomination.

const DUSK_UNIT: f64 = 1_000_000_000.0;

/// The minimum increment of Dusk.
pub const LUX: Dusk = dusk(1.0 / DUSK_UNIT);

/// The Dusk denomination. Use the [`dusk`] function to convert from floating
/// point format, and the [`from_dusk`] function to convert back to Dusk.
///
/// Values of Dusk should *never* be assigned directly. Instead they should use
/// a call to the [`dusk`] function. If increments of the smallest denomination
/// are desired, the [`LUX`] constant can be used.
pub type Dusk = u64;

/// Converts from floating point format to Dusk.
pub const fn dusk(value: f64) -> Dusk {
    (value * DUSK_UNIT) as Dusk
}

/// Converts from Dusk to floating point format.
pub const fn from_dusk(dusk: Dusk) -> f64 {
    dusk as f64 / DUSK_UNIT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_to_dusk() {
        let value = 5f64;
        let dusk_value = dusk(value);

        assert_eq!(value, from_dusk(dusk_value));
    }

    #[test]
    fn lux_is_one() {
        assert_eq!(LUX, 1);
    }
}
