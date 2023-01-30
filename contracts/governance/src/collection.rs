// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use alloc::vec::Vec;
use canonical::CanonError;
use canonical_derive::Canon;

#[derive(Clone, Canon, Debug)]
pub struct Collection<K, V> {
    data: Vec<(K, V)>,
}

impl<K: PartialEq, V: PartialEq> Collection<K, V> {
    // Methods return a result to keep things consistent with the map methods
    pub fn get(&self, key: &K) -> Result<Option<&V>, CanonError> {
        Ok(self.data.iter().find_map(|(k, v)| (k == key).then_some(v)))
    }

    pub fn insert(&mut self, key: K, value: V) -> Result<(), CanonError> {
        if let Some(pos) = self.data.iter().position(|(k, _)| k == &key) {
            self.data[pos] = (key, value)
        } else {
            self.data.push((key, value))
        }

        Ok(())
    }

    pub fn remove(&mut self, key: &K) -> Result<(), CanonError> {
        self.data.retain(|(k, _)| k != key);

        Ok(())
    }
}

impl<K, V> Default for Collection<K, V> {
    fn default() -> Self {
        Self { data: Vec::new() }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn dummy() -> Collection<u8, u8> {
        let mut collections: Collection<u8, u8> = Default::default();

        collections.insert(1, 20).unwrap();
        collections.insert(12, 120).unwrap();

        collections
    }

    #[test]
    fn get() {
        // this is also a test of insert
        let data = dummy();

        assert_eq!(*data.get(&1).unwrap().unwrap(), 20);
        assert_eq!(*data.get(&12).unwrap().unwrap(), 120);
    }

    #[test]
    fn insert() {
        let mut data = dummy();

        // updates the value
        data.insert(1, 22).unwrap();
        data.insert(4, 90).unwrap();

        assert_eq!(*data.get(&1).unwrap().unwrap(), 22);
        assert_eq!(*data.get(&4).unwrap().unwrap(), 90);
    }

    #[test]
    #[should_panic]
    fn remove() {
        let mut data = dummy();

        data.remove(&12).unwrap();

        data.get(&12).unwrap().unwrap();
    }
}
