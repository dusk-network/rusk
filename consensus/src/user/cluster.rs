// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::btree_map::Iter;
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub struct Cluster<T>(BTreeMap<T, usize>);

impl<T> Cluster<T>
where
    T: Default + std::cmp::Ord + Clone + std::fmt::Debug,
{
    pub(crate) fn new() -> Self {
        Self(Default::default())
    }

    /// total_occurrences return the sum of all the values.
    pub fn total_occurrences(&self) -> usize {
        self.0.values().sum()
    }

    /// Adds key with specified weight. Weight per key can be set only once.
    ///
    /// Return None if `weight` is 0 or key is already set.
    pub fn add(&mut self, key: &T, weight: usize) -> Option<usize> {
        if weight == 0 {
            return None;
        }
        if self.0.contains_key(key) {
            // already updated
            return None;
        }

        self.0.insert(key.clone(), weight);
        Some(weight)
    }

    pub fn iter(&self) -> Iter<T, usize> {
        self.0.iter()
    }

    pub fn into_vec(self) -> Vec<(T, usize)> {
        self.0.into_iter().collect()
    }

    pub fn contains_key(&self, key: &T) -> bool {
        self.0.contains_key(key)
    }
}

#[cfg(test)]
mod tests {
    use crate::user::cluster::Cluster;

    #[test]
    pub fn test_set_weight() {
        let mut a = Cluster::new();

        a.add(&'a', 3);
        a.add(&'b', 0);
        a.add(&'b', 11);
        assert_eq!(a.total_occurrences(), 14);

        let res = a.add(&'b', 1);
        assert!(res.is_none());
        assert_eq!(a.total_occurrences(), 14);
    }
}
