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

#[derive(Debug, Clone)]
pub struct Set<V> {
    data: Vec<V>,
}

#[allow(dead_code)]
impl<K: PartialEq, V: PartialEq> Map<K, V> {
    pub const fn new() -> Self {
        Self { data: Vec::new() }
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
        F: Fn(&V) -> bool
    {
        self.data.iter().find_map(|(_, v)| f(v).then_some(v))
    }
}

#[allow(dead_code)]
impl<V: PartialEq> Set<V> {
    pub const fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn get(&self, value: &V) -> Option<&V> {
        self.data.iter().find(|&v| v == value)
    }

    pub fn contains(&self, value: &V) -> bool {
        self.data.iter().any(|v| v == value)
    }

    pub fn insert(&mut self, value: V) -> bool {
        if self.contains(&value) {
            return false;
        }

        self.data.push(value);
        true
    }

    pub fn remove(&mut self, value: &V) {
        self.data.retain(|v| v != value);
    }
}

impl<K, V> Default for Map<K, V> {
    fn default() -> Self {
        Self { data: Vec::new() }
    }
}

impl<V> Default for Set<V> {
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

    #[test]
    fn test_set() {
        let mut data = Set::<u8>::default();

        assert!(!data.contains(&1), "1 is not in the set");
        assert!(!data.contains(&12), "12 is not in the set");

        data.insert(12);

        assert!(!data.contains(&1), "1 is still not in the set");
        assert!(data.contains(&12), "12 is in the set");

        data.remove(&12);

        assert!(!data.contains(&1), "1 is still not in the set");
        assert!(!data.contains(&12), "12 is removed from the set");

        data.remove(&10);

        assert!(!data.contains(&10), "10 is not in the set");
    }
}
