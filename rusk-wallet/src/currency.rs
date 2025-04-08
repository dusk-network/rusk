// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

//! Module to help with currency conversions.

use core::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::num::ParseFloatError;
use std::ops::{Add, Deref, Div, Mul, Sub};
use std::str::FromStr;

use dusk_core::{dusk, from_dusk};

use crate::Error;

/// The underlying unit of Dusk
pub type Lux = u64;

/// Denomination for DUSK
#[derive(Copy, Clone, Debug, Eq)]
pub struct Dusk(Lux);

impl Dusk {
    /// The smallest value that can be represented by Dusk currency
    pub const MIN: Dusk = Dusk(0);
    /// The largest value that can be represented by Dusk currency
    pub const MAX: Dusk = Dusk(dusk(f64::MAX / dusk(1.0) as f64));

    /// Returns a new Dusk based on the [Lux] given
    pub const fn new(lux: Lux) -> Dusk {
        Self(lux)
    }
}

/// Core ops
/// Implementations of Addition, Subtraction, Multiplication,
/// Division, and Comparison operators for Dusk

/// Addition
impl Add for Dusk {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl Add<Lux> for Dusk {
    type Output = Self;
    fn add(self, other: Lux) -> Self {
        Self(self.0 + other)
    }
}

/// Subtraction
impl Sub for Dusk {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }
}

impl Sub<Lux> for Dusk {
    type Output = Self;
    fn sub(self, other: Lux) -> Self {
        Self(self.0 - other)
    }
}

/// Multiplication
impl Mul for Dusk {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        let a = from_dusk(self.0);
        let b = from_dusk(other.0);
        Self(dusk(a * b))
    }
}

impl Mul<Lux> for Dusk {
    type Output = Self;
    fn mul(self, other: Lux) -> Self {
        let a = from_dusk(self.0);
        let b = from_dusk(other);
        Self(dusk(a * b))
    }
}

/// Division
impl Div for Dusk {
    type Output = Self;
    fn div(self, other: Self) -> Self {
        Self(dusk(self.0 as f64 / other.0 as f64))
    }
}

impl Div<Lux> for Dusk {
    type Output = Self;
    fn div(self, other: Lux) -> Self {
        Self(dusk(self.0 as f64 / other as f64))
    }
}

/// Equality
impl Hash for Dusk {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}
impl PartialEq for Dusk {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl PartialEq<Lux> for Dusk {
    fn eq(&self, other: &Lux) -> bool {
        self.0 == *other
    }
}
impl PartialEq<f64> for Dusk {
    fn eq(&self, other: &f64) -> bool {
        self.0 == dusk(*other)
    }
}

/// Comparison
impl Ord for Dusk {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(other)
    }
}

impl PartialOrd for Dusk {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialOrd<Lux> for Dusk {
    fn partial_cmp(&self, other: &Lux) -> Option<Ordering> {
        self.0.partial_cmp(other)
    }
}
impl PartialOrd<f64> for Dusk {
    fn partial_cmp(&self, other: &f64) -> Option<Ordering> {
        self.0.partial_cmp(&dusk(*other))
    }
}

/// Conversion ops
/// Convenient conversion of primitives to and from Dusk

/// Floats are used directly as Dusk value
impl TryFrom<f64> for Dusk {
    type Error = Error;
    fn try_from(val: f64) -> Result<Self, Error> {
        if val < 0.0 {
            return Err(Error::Conversion(
                "Dusk type does not support negative values".to_string(),
            ));
        }
        Ok(Self(dusk(val)))
    }
}

impl From<Dusk> for f64 {
    fn from(val: Dusk) -> f64 {
        from_dusk(*val)
    }
}

impl From<&Dusk> for f64 {
    fn from(val: &Dusk) -> f64 {
        (*val).into()
    }
}

/// Lux represent Dusk in their underlying unit type
impl From<Lux> for Dusk {
    fn from(lux: Lux) -> Self {
        Self(lux)
    }
}

/// Strings are parsed as Dusk values (floats)
impl FromStr for Dusk {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parse_result = f64::from_str(s).map_err(|e: ParseFloatError| {
            Error::Conversion(format!(
                "Failed to parse Dusk from string: {}",
                e
            ))
        })?;

        Dusk::try_from(parse_result)
    }
}

/// Dusk derefs into its underlying Lux amount
impl Deref for Dusk {
    type Target = Lux;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Display
/// Let the user print stuff
impl fmt::Display for Dusk {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let v: f64 = self.into();
        f64::fmt(&v, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basics() {
        let one = Dusk::try_from(1.0).unwrap();
        let dec = Dusk::try_from(2.25).unwrap();
        assert_eq!(one, 1.0);
        assert_eq!(dec, 2.25);
        assert_eq!(Dusk::MIN, 0);
        assert_eq!(Dusk::MIN, Dusk::try_from(0.0).unwrap());
    }

    #[test]
    fn compare_dusk() {
        let one = Dusk::try_from(1.0).unwrap();
        let two = Dusk::try_from(2.0).unwrap();
        let dec_a = Dusk::try_from(0.00025).unwrap();
        let dec_b = Dusk::try_from(0.00190).unwrap();
        assert!(one == one);
        assert!(one != two);
        assert!(one < two);
        assert!(one <= two);
        assert!(one >= one);
        assert!(dec_a < dec_b);
        assert!(one > dec_b);
    }

    #[test]
    fn ops_dusk_dusk() {
        let one = Dusk::try_from(1.0).unwrap();
        let two = Dusk::try_from(2.0).unwrap();
        let three = Dusk::try_from(3.0).unwrap();
        assert_eq!(one + two, three);
        assert_eq!(three - two, one);
        assert_eq!(one * one, one);
        assert_eq!(two * one, two);
        assert_eq!(two / one, two);
        let point_five = Dusk::try_from(0.5).unwrap();
        assert_eq!(one / two, point_five);
        assert_eq!(point_five * point_five, Dusk::try_from(0.25).unwrap());
    }

    #[test]
    fn ops_dusk_lux() {
        let one = Dusk::try_from(1.0).unwrap();
        let one_dusk = 1000000000;
        assert_eq!(one + one_dusk, 2.0);
        assert_eq!(one - one_dusk, 0.0);
        assert_eq!(one * one_dusk, 1.0);
        assert_eq!(one / one_dusk, 1.0);
    }

    #[test]
    fn conversions() {
        let my_float = 35.049;
        let dusk: Dusk = my_float.try_into().unwrap();
        assert_eq!(dusk, my_float);
        let one_dusk = 1_000_000_000u64;
        let dusk: Dusk = one_dusk.try_into().unwrap();
        assert_eq!(dusk, 1.0);
        assert_eq!(*dusk, one_dusk);
        let dusk = Dusk::from_str("69.420").unwrap();
        assert_eq!(dusk, 69.420);
        let float: f64 = dusk.try_into().unwrap();
        assert_eq!(float, 69.420);
        let borrowed = &Dusk(one_dusk);
        let float: f64 = borrowed.try_into().unwrap();
        assert_eq!(float, 1.0);
        let zero = 0;
        assert_eq!(Dusk::try_from(zero).unwrap(), 0);
        let zero = 0.0;
        assert_eq!(Dusk::try_from(zero).unwrap(), 0.0);
    }

    #[test]
    #[should_panic]
    fn overflow() {
        let ten = Dusk::try_from(10.0).unwrap();
        let _ = Dusk::MAX + ten;
    }

    #[test]
    fn negative_dusk() {
        assert!(Dusk::try_from(-1.0).is_err());
    }

    #[test]
    #[should_panic]
    fn negative_result() {
        let one = Dusk::try_from(1.0).unwrap();
        let two = Dusk::try_from(2.0).unwrap();
        let _ = one - two;
    }
}
