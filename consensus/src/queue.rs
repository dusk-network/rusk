// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::collections::BTreeMap;
use std::fmt::Debug;

type StepMap<T> = BTreeMap<u16, Vec<T>>;
type RoundMap<T> = BTreeMap<u64, StepMap<T>>;

#[derive(Debug, Default)]
pub struct MsgRegistry<T: ?Sized>(RoundMap<T>)
where
    T: Debug + Clone;

/// A message registry that stores messages based on their round and step.
impl<T: Debug + Clone> MsgRegistry<T> {
    /// Inserts a message into the registry based on its round and step.
    pub fn put_msg(&mut self, round: u64, step: u16, msg: T) {
        self.0
            .entry(round)
            .or_default()
            .entry(step)
            .or_default()
            .push(msg);
    }

    /// Drains and returns all messages that belong to the specified round and
    /// step.
    pub fn drain_msg_by_round_step(
        &mut self,
        round: u64,
        step: u16,
    ) -> Option<Vec<T>> {
        self.0
            .get_mut(&round)
            .and_then(|r| r.remove_entry(&step).map(|(_, v)| v))
    }

    /// Removes all messages that belong to the specified round.
    pub fn remove_msgs_by_round(&mut self, round: u64) {
        if let Some(r) = self.0.get_mut(&round) {
            r.clear();
        };

        self.0.remove(&round);
    }

    /// Returns the total number of messages in the registry.
    pub fn msg_count(&self) -> usize {
        self.0
            .values()
            .map(|round| round.values().map(|items| items.len()).sum::<usize>())
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use crate::queue::MsgRegistry;

    #[test]
    pub fn test_push_event() {
        #[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
        struct Item(i32);

        let round = 55555;

        let mut queue = MsgRegistry::<Item>::default();
        queue.put_msg(round, 2, Item(5));
        queue.put_msg(round, 2, Item(4));
        queue.put_msg(round, 2, Item(3));

        assert_eq!(queue.msg_count(), 3);
        assert!(queue.drain_msg_by_round_step(round, 3).is_none());
        assert!(queue.drain_msg_by_round_step(4444, 2).is_none());

        for i in 1..100 {
            queue.put_msg(4444, i as u16, Item(i));
        }

        assert_eq!(queue.msg_count(), 100 + 2);
        assert_eq!(
            queue.drain_msg_by_round_step(round, 2).unwrap(),
            vec![Item(5), Item(4), Item(3)],
        );
        assert_eq!(queue.msg_count(), 99);

        queue.remove_msgs_by_round(4444);
        assert_eq!(queue.msg_count(), 0);
        assert!(queue.drain_msg_by_round_step(round, 2).is_none());
    }
}
