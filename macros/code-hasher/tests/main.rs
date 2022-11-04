// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::prelude::*;

#[derive(Debug, Default)]
pub struct TestCircuit {}

#[code_hasher::hash(name = "CIRCUIT_ID")]
impl Circuit for TestCircuit {
    fn circuit<C: Composer>(&self, _composer: &mut C) -> Result<(), Error> {
        unimplemented!()
    }
}

#[derive(Debug, Default)]
pub struct AnotherTestCircuit {}

#[code_hasher::hash(name = "CIRCUIT_ID")]
impl Circuit for AnotherTestCircuit {
    fn circuit<C: Composer>(&self, _composer: &mut C) -> Result<(), Error> {
        unimplemented!()
    }
}

#[code_hasher::hash(name = "MAJOR_HASH_1", version = "1.0.1")]
impl TestCircuit {}

#[code_hasher::hash(name = "MAJOR_HASH_2", version = "1.1.1")]
impl TestCircuit {}

#[code_hasher::hash(name = "MAJOR_HASH_3", version = "2.1.1")]
impl TestCircuit {}

#[code_hasher::hash(name = "MINOR_HASH_1", version = "0.1.0")]
impl TestCircuit {}

#[code_hasher::hash(name = "MINOR_HASH_2", version = "0.1.1")]
impl TestCircuit {}

#[code_hasher::hash(name = "MINOR_HASH_3", version = "0.0.0")]
impl TestCircuit {}

mod tests {
    use super::*;

    #[test]
    fn plonk_testcase_works() {
        assert_eq!(
            &TestCircuit::CIRCUIT_ID,
            &[
                204, 39, 45, 48, 124, 51, 199, 86, 58, 196, 113, 171, 253, 88,
                69, 13, 175, 162, 92, 76, 240, 138, 151, 178, 212, 136, 233,
                126, 161, 146, 54, 214
            ]
        );
    }

    #[test]
    fn diff_struct_is_diff_hash() {
        assert_ne!(&TestCircuit::CIRCUIT_ID, &AnotherTestCircuit::CIRCUIT_ID);
    }

    #[test]
    fn version_changes_as_expected() {
        assert_eq!(&TestCircuit::MAJOR_HASH_1, &TestCircuit::MAJOR_HASH_2);
        assert_ne!(&TestCircuit::MAJOR_HASH_1, &TestCircuit::MAJOR_HASH_3);

        assert_eq!(&TestCircuit::MINOR_HASH_1, &TestCircuit::MINOR_HASH_2);
        assert_ne!(&TestCircuit::MINOR_HASH_1, &TestCircuit::MINOR_HASH_3);
    }
}
