// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::messages::Message;
use std::collections::BTreeMap;
use std::fmt::Debug;
use tokio::sync::Mutex;

#[derive(PartialEq, Debug)]
pub enum Error {
    NotFound,
}

#[derive(Debug, Default)]
pub struct Queue<T>(Mutex<(BTreeMap<u64, BTreeMap<u8, Vec<T>>>, usize)>)
where
    T: Debug + Message + Copy;

impl<T: Debug + Message + Copy> Queue<T> {
    pub async fn put_event(&mut self, round: u64, step: u8, msg: T) {
        let mut queue = self.0.lock().await;

        // insert entry [round] -> [u8 -> Vec<T>]
        queue
            .0
            .entry(round)
            .or_insert(BTreeMap::new())
            .entry(step)
            .or_insert(vec![])
            .push(msg);

        queue.1 += 1;
    }

    pub async fn get_events(&self, round: u64, step: u8) -> Result<Vec<T>, Error> {
        let mut queue = self.0.lock().await;

        // TODO: here we should consider to consume the array of events instead of returning a deep copy.

        match queue.0.get(&round) {
            Some(r) => match r.get(&step) {
                None => Err(Error::NotFound),
                Some(v) => Ok(v.clone()),
            },
            None => Err(Error::NotFound),
        }
    }

    pub async fn clear(&self, round: u64) -> Result<(), Error> {
        let mut queue = self.0.lock().await;

        match queue.0.get_mut(&round) {
            Some(r) => {
                r.clear();
                Ok(())
            }
            None => Err(Error::NotFound),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::messages::Message;
    use crate::queue::{Error, Queue};
    use crate::user::provisioners::PublicKey;
    use crate::user::sortition::{create_sortition_hash, generate_sortition_score};
    use hex_literal::hex;
    use num_bigint::BigInt;

    #[test]
    pub fn test_push_event() {
        #[derive(Copy, Clone, Debug, Default, PartialEq)]
        struct Item(i32);
        impl Message for Item {
            fn compare(&self, round: u64, step: u8) -> bool {
                false
            }

            fn get_pubkey_bls(&self) -> PublicKey {
                PublicKey::default()
            }
        }

        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let round = 55555;

                let mut queue = Queue::<Item>::default();
                queue.put_event(round, 2, Item(5)).await;
                queue.put_event(round, 2, Item(4)).await;
                queue.put_event(round, 2, Item(3)).await;

                assert_eq!(
                    queue.get_events(round, 3).await.unwrap_err(),
                    Error::NotFound
                );

                assert_eq!(
                    queue.get_events(4444, 2).await.unwrap_err(),
                    Error::NotFound
                );

                for i in 1..100 {
                    queue.put_event(4444, i as u8, Item(i)).await;
                }

                assert_eq!(
                    queue.get_events(round, 2).await.unwrap(),
                    vec![Item(5), Item(4), Item(3)],
                );

                let _ = queue.clear(round).await;

                assert_eq!(
                    queue.get_events(round, 2).await.unwrap_err(),
                    Error::NotFound,
                );
            });
    }
}
