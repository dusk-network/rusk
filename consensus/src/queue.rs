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

    /// Removes all messages that belong to a round greater than the specified.
    pub fn remove_msgs_greater_than(&mut self, round: u64) {
        self.0.split_off(&round);
    }

    /// Removes all messages that do not belong to the range (closed interval)
    /// of keys
    pub fn remove_msgs_out_of_range(&mut self, start_round: u64, offset: u64) {
        let end_round = start_round + offset;

        self.0 = self
            .0
            .split_off(&start_round)
            .into_iter()
            .filter(|(k, _)| *k <= end_round)
            .collect();
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
    fn test_push_event() {
        #[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
        struct Item(i32);

        let round = 55555;

        let mut reg = MsgRegistry::<Item>::default();
        reg.put_msg(round, 2, Item(5));
        reg.put_msg(round, 2, Item(4));
        reg.put_msg(round, 2, Item(3));

        assert_eq!(reg.msg_count(), 3);
        assert!(reg.drain_msg_by_round_step(round, 3).is_none());
        assert!(reg.drain_msg_by_round_step(4444, 2).is_none());

        for i in 1..100 {
            reg.put_msg(4444, i as u16, Item(i));
        }

        assert_eq!(reg.msg_count(), 100 + 2);
        assert_eq!(
            reg.drain_msg_by_round_step(round, 2).unwrap(),
            vec![Item(5), Item(4), Item(3)],
        );
        assert_eq!(reg.msg_count(), 99);

        reg.remove_msgs_by_round(4444);
        assert_eq!(reg.msg_count(), 0);
        assert!(reg.drain_msg_by_round_step(round, 2).is_none());
    }

    #[test]
    fn test_remove() {
        #[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
        struct Item(i32);

        let round = 100;

        let mut reg = MsgRegistry::<Item>::default();
        reg.put_msg(round + 1, 1, Item(1));
        reg.put_msg(round + 2, 1, Item(1));
        reg.put_msg(round + 3, 1, Item(1));
        reg.put_msg(round, 1, Item(1));

        reg.remove_msgs_greater_than(round + 2);

        assert!(reg.drain_msg_by_round_step(round, 1).is_some());
        assert!(reg.drain_msg_by_round_step(round + 1, 1).is_some());

        assert!(reg.drain_msg_by_round_step(round + 2, 1).is_none());
        assert!(reg.drain_msg_by_round_step(round + 3, 1).is_none());

        assert_eq!(reg.msg_count(), 0);
    }

    #[test]
    fn test_remove_msgs_out_of_range() {
        #[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
        struct Item(i32);

        let round = 100;

        let mut reg = MsgRegistry::<Item>::default();
        reg.put_msg(round + 1, 1, Item(1));
        reg.put_msg(round + 2, 1, Item(1));
        reg.put_msg(round * 3, 1, Item(1));
        reg.put_msg(round, 1, Item(1));
        assert_eq!(reg.msg_count(), 4);

        reg.remove_msgs_out_of_range(round + 1, 1);
        assert_eq!(reg.msg_count(), 2);

        assert!(reg.drain_msg_by_round_step(round, 1).is_none());
        assert!(reg.drain_msg_by_round_step(round * 3, 1).is_none());

        assert!(reg.drain_msg_by_round_step(round + 1, 1).is_some());
        assert!(reg.drain_msg_by_round_step(round + 2, 1).is_some());
    }
}
