// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::borrow::Borrow;

use zeroize::Zeroize;

pub type ZeroizingBytes = ZeroizingVec<u8>;

/// The purpose of this struct is to provide a testable
/// zeroize-on-drop wrapper around a non-reallocating vector - this
/// is to ensure that the underlying data is stored in a single location
/// on the heap and will be zeroized when dropped.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Zeroize)]
pub struct ZeroizingVec<T: Zeroize>(Vec<T>);

impl<T: Zeroize> Drop for ZeroizingVec<T> {
    fn drop(&mut self) {
        self.0.zeroize();
        assert!(self.0.is_empty(), "Zeroization failed");
    }
}

impl From<String> for ZeroizingVec<u8> {
    fn from(s: String) -> Self {
        Self(s.into_bytes())
    }
}

impl From<Vec<u8>> for ZeroizingVec<u8> {
    fn from(v: Vec<u8>) -> Self {
        Self(v)
    }
}

impl Borrow<[u8]> for ZeroizingVec<u8> {
    fn borrow(&self) -> &[u8] {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::BorrowMut;
    use std::cell::RefCell;
    use std::mem::ManuallyDrop;
    use std::sync::Arc;

    use super::*;

    struct AppendOnZeroize {
        val: i32,
        append_to: Arc<RefCell<ManuallyDrop<Vec<i32>>>>,
    }

    impl Zeroize for AppendOnZeroize {
        fn zeroize(&mut self) {
            let mut append_to = RefCell::borrow_mut(&self.append_to);
            append_to.push(self.val);
        }
    }

    #[test]
    fn test_zeroizing_bytes() {
        let vec = Arc::new(RefCell::new(ManuallyDrop::new(vec![])));
        let z_vec = ZeroizingVec(vec![
            AppendOnZeroize {
                val: 1,
                append_to: vec.clone(),
            },
            AppendOnZeroize {
                val: 2,
                append_to: vec.clone(),
            },
            AppendOnZeroize {
                val: 3,
                append_to: vec.clone(),
            },
        ]);

        drop(z_vec);

        let vec = RefCell::take(&vec);
        let vec = ManuallyDrop::into_inner(vec);
        assert_eq!(vec, vec![1i32, 2, 3]);
    }
}
