// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::vec::Vec;

#[derive(Debug, Clone)]
pub struct Map<K, V> {
    data: Vec<(K, V)>,
}

#[allow(dead_code)]
impl<K: PartialEq, V: PartialEq> Map<K, V> {
    pub const fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.data.iter().find_map(|(k, v)| (k == key).then_some(v))
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.data
            .iter_mut()
            .find_map(|(k, v)| (k == key).then_some(v))
    }

    pub fn insert(&mut self, key: K, value: V) {
        if let Some(pos) = self.data.iter().position(|(k, _)| k == &key) {
            self.data[pos] = (key, value)
        } else {
            self.data.push((key, value))
        }
    }

    pub fn remove(&mut self, key: &K) {
        self.data.retain(|(k, _)| k != key);
    }

    pub fn find<F>(&self, f: F) -> Option<&V>
    where
        F: Fn(&V) -> bool,
    {
        self.data.iter().find_map(|(_, v)| f(v).then_some(v))
    }

    pub fn filter<F>(&self, f: F) -> impl Iterator<Item = &V>
    where
        F: Fn(&V) -> bool,
    {
        self.data.iter().filter_map(move |(_, v)| f(v).then_some(v))
    }

    pub fn entries_filter<F>(&self, f: F) -> impl Iterator<Item = &(K, V)>
    where
        F: Fn((&K, &V)) -> bool,
    {
        self.data.iter().filter(move |(k, v)| f((k, v)))
    }
}

impl<K, V> Default for Map<K, V> {
    fn default() -> Self {
        Self { data: Vec::new() }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_map() {
        let mut data = Map::<u8, u8>::default();

        assert!(data.get(&1).is_none());
        assert!(data.get(&12).is_none());

        data.insert(12, 0);

        assert!(data.get(&1).is_none());
        assert!(data.get(&12).is_some());

        data.remove(&12);

        assert!(data.get(&1).is_none());
        assert!(data.get(&12).is_none());
    }
}
