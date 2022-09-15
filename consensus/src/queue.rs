// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::BTreeMap;
use std::fmt::Debug;

#[derive(PartialEq, Eq, Debug)]
pub enum Error {
    NotFound,
}

type Map<T> = BTreeMap<u8, Vec<T>>;

/// Atomic message queue to store messages by round and step
#[derive(Debug, Default)]
pub struct Queue<T: ?Sized>(BTreeMap<u64, Map<T>>, usize)
where
    T: Debug + Clone;

impl<T: Debug + Clone> Queue<T> {
    pub fn put_event(&mut self, round: u64, step: u8, msg: T) {
        let queue = &mut self.0;

        // insert entry [round] -> [u8 -> Vec<T>]
        queue
            .entry(round)
            .or_insert(BTreeMap::new())
            .entry(step)
            .or_insert(vec![])
            .push(msg);

        self.1 += 1;
    }

    pub fn get_events(&self, round: u64, step: u8) -> Result<Vec<T>, Error> {
        let _queue = &self.0;

        // TODO: here we should consider to consume the array of events instead of returning a deep copy.

        match self.0.get(&round) {
            Some(r) => match r.get(&step) {
                None => Err(Error::NotFound),
                Some(v) => Ok(v.clone()),
            },
            None => Err(Error::NotFound),
        }
    }

    pub fn clear(&mut self, round: u64) {
        let queue = &mut self.0;

        match queue.get_mut(&round) {
            Some(r) => {
                r.clear();
            }
            None => {}
        };

        queue.remove(&round);
    }
}

#[cfg(test)]
mod tests {
    use crate::queue::{Error, Queue};

    #[test]
    pub fn test_push_event() {
        #[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
        struct Item(i32);

        let round = 55555;

        let mut queue = Queue::<Item>::default();
        queue.put_event(round, 2, Item(5));
        queue.put_event(round, 2, Item(4));
        queue.put_event(round, 2, Item(3));

        assert_eq!(queue.get_events(round, 3).unwrap_err(), Error::NotFound);

        assert_eq!(queue.get_events(4444, 2).unwrap_err(), Error::NotFound);

        for i in 1..100 {
            queue.put_event(4444, i as u8, Item(i));
        }

        assert_eq!(
            queue.get_events(round, 2).unwrap(),
            vec![Item(5), Item(4), Item(3)],
        );

        queue.clear(round);

        assert_eq!(queue.get_events(round, 2).unwrap_err(), Error::NotFound,);
    }
}
