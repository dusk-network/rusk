// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::BTreeMap;
use std::fmt::Debug;

type StepMap<T> = BTreeMap<u16, Vec<T>>;
type RoundMap<T> = BTreeMap<u64, StepMap<T>>;

/// Atomic message queue to store messages by round and step
#[derive(Debug, Default)]
pub struct Queue<T: ?Sized>(RoundMap<T>, usize)
where
    T: Debug + Clone;

impl<T: Debug + Clone> Queue<T> {
    pub fn put_event(&mut self, round: u64, step: u16, msg: T) {
        // insert entry [round] -> [u8 -> Vec<T>]
        self.0
            .entry(round)
            .or_default()
            .entry(step)
            .or_default()
            .push(msg);

        self.1 += 1;
    }

    pub fn drain_events(&mut self, round: u64, step: u16) -> Option<Vec<T>> {
        self.0
            .get_mut(&round)
            .and_then(|r| r.remove_entry(&step).map(|(_, v)| v))
    }

    pub fn clear_round(&mut self, round: u64) {
        if let Some(r) = self.0.get_mut(&round) {
            r.clear();
        };

        self.0.remove(&round);
    }
}

#[cfg(test)]
mod tests {
    use crate::queue::Queue;

    #[test]
    pub fn test_push_event() {
        #[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
        struct Item(i32);

        let round = 55555;

        let mut queue = Queue::<Item>::default();
        queue.put_event(round, 2, Item(5));
        queue.put_event(round, 2, Item(4));
        queue.put_event(round, 2, Item(3));

        assert!(queue.drain_events(round, 3).is_none());

        assert!(queue.drain_events(4444, 2).is_none());

        for i in 1..100 {
            queue.put_event(4444, i as u16, Item(i));
        }

        assert_eq!(
            queue.drain_events(round, 2).unwrap(),
            vec![Item(5), Item(4), Item(3)],
        );

        queue.clear_round(round);

        assert!(queue.drain_events(round, 2).is_none());
    }
}
