// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use dusk_plonk::prelude::*;

#[derive(Debug, Default)]
pub struct TestCircuit1 {}

#[code_hasher::hash(CIRCUIT_ID, version = "0.1.0")]
impl Circuit for TestCircuit1 {
    fn gadget(&mut self, composer: &mut StandardComposer) -> Result<(), Error> {
        unimplemented!()
    }

    fn padded_circuit_size(&self) -> usize {
        1 << 11
    }
}

#[derive(Debug, Default)]
pub struct TestCircuit2 {}

#[code_hasher::hash(CIRCUIT_ID, version = "0.1.0")]
impl Circuit for TestCircuit2 {
    fn gadget(&mut self, composer: &mut StandardComposer) -> Result<(), Error> {
        unimplemented!()
    }

    fn padded_circuit_size(&self) -> usize {
        1 << 11
    }
}

#[code_hasher::hash(SOME_CONST_NAME, version = "0.1.0")]
pub mod testing_module {

    pub fn this_does_something() -> [u8; 32] {
        SOME_CONST_NAME
    }
}

#[code_hasher::hash(SOME_CONST_NAME)]
pub mod testing_module_without_version {

    pub fn this_does_something() -> [u8; 32] {
        SOME_CONST_NAME
    }
}

mod tests {
    use super::*;

    #[test]
    fn plonk_testcase_works() {
        assert_eq!(
            &TestCircuit1::CIRCUIT_ID,
            &[
                75, 161, 53, 125, 63, 238, 118, 194, 248, 174, 128, 90, 214,
                72, 157, 0, 184, 26, 251, 147, 220, 253, 98, 165, 250, 188, 87,
                9, 194, 33, 108, 91
            ]
        );
    }

    #[test]
    fn diff_version_is_diff_hash() {
        assert_ne!(&TestCircuit1::CIRCUIT_ID, &TestCircuit2::CIRCUIT_ID,);
    }

    #[test]
    fn custom_mods_and_names_work() {
        assert_ne!(testing_module::this_does_something(), [0u8; 32]);
        assert_ne!(
            testing_module::this_does_something(),
            testing_module_without_version::this_does_something()
        );
    }
}
