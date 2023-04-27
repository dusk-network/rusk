// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::vec::Vec;

// todo: this code is duplicated here from the governance contract, remove this
// duplication

#[derive(Debug, Clone)]
pub struct Map<K, V> {
    data: Vec<(K, V)>,
}

impl<K: PartialEq, V: PartialEq> Map<K, V> {
    pub const fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.data.iter().find_map(|(k, v)| (k == key).then_some(v))
    }

    #[allow(dead_code)]
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.data
            .iter_mut()
            .find_map(|(k, v)| (k == key).then_some(v))
    }

    #[allow(dead_code)]
    pub fn insert(&mut self, key: K, value: V) {
        if let Some(pos) = self.data.iter().position(|(k, _)| k == &key) {
            self.data[pos] = (key, value)
        } else {
            self.data.push((key, value))
        }
    }

    #[allow(dead_code)]
    pub fn remove(&mut self, key: &K) {
        self.data.retain(|(k, _)| k != key);
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
