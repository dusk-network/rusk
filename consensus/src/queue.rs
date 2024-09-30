// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use node_data::message::Message;
use std::collections::{BTreeMap, VecDeque};
use std::fmt::Debug;
use tracing::warn;

type StepMap<T> = BTreeMap<u8, VecDeque<T>>;
type RoundMap<T> = BTreeMap<u64, StepMap<T>>;

const MAX_MESSAGES_PER_QUEUE: usize = 1000;

#[derive(Debug, Default)]
pub struct MsgRegistry<T: QueueMessage>(RoundMap<T>);

pub trait QueueMessage: Debug + Clone {
    fn step(&self) -> u8;

    fn round(&self) -> u64;
}

impl QueueMessage for Message {
    fn round(&self) -> u64 {
        self.header.round
    }
    fn step(&self) -> u8 {
        self.get_step()
    }
}

/// A message registry that stores messages based on their round and step.
impl<T: QueueMessage> MsgRegistry<T> {
    /// Inserts a message into the registry based on its round and step.
    pub fn put_msg(&mut self, msg: T) {
        let round = msg.round();
        let step = msg.step();
        let vec = self
            .0
            .entry(round)
            .or_default()
            .entry(step)
            .or_insert(VecDeque::with_capacity(MAX_MESSAGES_PER_QUEUE));

        if vec.len() == vec.capacity() {
            warn!("queue ({}, {}) is full, dropping", round, step);
            vec.pop_front();
        }

        vec.push_back(msg);
    }

    /// Drains and returns all messages that belong to the specified round and
    /// step.
    pub fn drain_msg_by_round_step(
        &mut self,
        round: u64,
        step: u8,
    ) -> Option<VecDeque<T>> {
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

    use super::QueueMessage;

    #[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
    struct Item(u64, u8, i32);

    impl QueueMessage for Item {
        fn round(&self) -> u64 {
            self.0
        }
        fn step(&self) -> u8 {
            self.1
        }
    }
    #[test]
    fn test_push_event() {
        let round = 55555;

        let mut reg = MsgRegistry::<Item>::default();
        reg.put_msg(Item(round, 2, 5));
        reg.put_msg(Item(round, 2, 4));
        reg.put_msg(Item(round, 2, 3));

        assert_eq!(reg.msg_count(), 3);
        assert!(reg.drain_msg_by_round_step(round, 3).is_none());
        assert!(reg.drain_msg_by_round_step(4444, 2).is_none());

        for i in 1..100 {
            reg.put_msg(Item(4444, i as u8, i));
        }

        assert_eq!(reg.msg_count(), 100 + 2);
        assert_eq!(
            reg.drain_msg_by_round_step(round, 2).unwrap(),
            vec![Item(round, 2, 5), Item(round, 2, 4), Item(round, 2, 3)],
        );
        assert_eq!(reg.msg_count(), 99);

        reg.remove_msgs_by_round(4444);
        assert_eq!(reg.msg_count(), 0);
        assert!(reg.drain_msg_by_round_step(round, 2).is_none());
    }

    #[test]
    fn test_remove_msgs_out_of_range() {
        let round = 100;

        let mut reg = MsgRegistry::<Item>::default();
        reg.put_msg(Item(round + 1, 1, 1));
        reg.put_msg(Item(round + 2, 1, 1));
        reg.put_msg(Item(round * 3, 1, 1));
        reg.put_msg(Item(round, 1, 1));
        assert_eq!(reg.msg_count(), 4);

        reg.remove_msgs_out_of_range(round + 1, 1);
        assert_eq!(reg.msg_count(), 2);

        assert!(reg.drain_msg_by_round_step(round, 1).is_none());
        assert!(reg.drain_msg_by_round_step(round * 3, 1).is_none());

        assert!(reg.drain_msg_by_round_step(round + 1, 1).is_some());
        assert!(reg.drain_msg_by_round_step(round + 2, 1).is_some());
    }
}
