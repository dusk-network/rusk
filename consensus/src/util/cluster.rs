// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::BTreeMap;

#[derive(Debug)]
pub struct Cluster<T>(pub BTreeMap<T, usize>);

impl<T> Cluster<T>
where
    T: Default + std::cmp::Ord + Copy + std::fmt::Debug,
{
    pub(crate) fn new() -> Self {
        Self(Default::default())
    }

    pub fn contains_key(&self, key: &T) -> bool {
        self.0.contains_key(key)
    }

    pub fn total_occurrences(&self) -> usize {
        let mut total = 0;
        for elem in self.0.iter() {
            total += elem.1;
        }

        total
    }

    /// set_weight can set weight only once.
    pub fn set_weight(&mut self, key: &T, weight: usize) -> Option<usize> {
        let entry = self.0.entry(*key).or_insert(0);
        if *entry > 0 {
            // already updated
            return None;
        }

        *entry = weight;
        Some(*entry)
    }

    /// get weight value, if key exists.
    pub fn get_weight(&self, key: &T) -> Option<usize> {
        match self.0.get_key_value(key) {
            Some(item) => Some(*item.1),
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::util::cluster::Cluster;

    #[test]
    pub fn test_set_weight() {
        let mut a = Cluster::<char>::new();

        a.set_weight(&'a', 3);
        a.set_weight(&'b', 11);
        assert_eq!(a.total_occurrences(), 14);

        let res = a.set_weight(&'b', 1);
        assert!(res == None);
    }
}
