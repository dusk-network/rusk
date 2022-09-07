// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::BTreeMap;
use tracing::trace;

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

    pub fn update_count(&mut self, key: &T, count: usize) -> usize {
        let entry = self.0.entry(*key).or_insert(0);
        *entry += count;
        *entry
    }

    pub fn trace(&self) -> () {
        for elem in self.0.iter() {
            trace!("item: {:#?}, size: {:#?}", elem.0, elem.1);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::util::cluster::Cluster;

    #[test]
    pub fn test_update_count() {
        let mut a = Cluster::<char>::new();

        a.update_count(&'a', 3);
        assert_eq!(a.update_count(&'a', 2), 5);

        a.update_count(&'b', 11);
        assert_eq!(a.update_count(&'a', 1), 6);
        assert_eq!(a.update_count(&'b', 1), 12);
        assert_eq!(a.total_occurrences(), 18);
    }
}
