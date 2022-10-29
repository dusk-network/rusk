// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::btree_map::Iter;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct Cluster<T>(BTreeMap<T, usize>);

impl<T> Cluster<T>
where
    T: Default + std::cmp::Ord + Copy + std::fmt::Debug,
{
    pub(crate) fn new() -> Self {
        Self(Default::default())
    }

    /// total_occurrences return the sum of all the values.
    pub fn total_occurrences(&self) -> usize {
        self.0.values().sum()
    }

    /// set_weight can set weight only once.
    pub fn set_weight(&mut self, key: &T, weight: usize) -> Option<usize> {
        if weight == 0 {
            return None;
        }
        if self.0.contains_key(key) {
            // already updated
            return None;
        }

        self.0.insert(*key, weight);
        Some(weight)
    }

    pub fn iter(&self) -> Iter<T, usize> {
        self.0.iter()
    }

    pub fn contains_key(&self, key: &T) -> bool {
        self.0.contains_key(key)
    }
}

#[cfg(test)]
mod tests {
    use crate::util::cluster::Cluster;

    #[test]
    pub fn test_set_weight() {
        let mut a = Cluster::new();

        a.set_weight(&'a', 3);
        a.set_weight(&'b', 0);
        a.set_weight(&'b', 11);
        assert_eq!(a.total_occurrences(), 14);

        let res = a.set_weight(&'b', 1);
        assert!(res == None);
        assert_eq!(a.total_occurrences(), 14);
    }
}
