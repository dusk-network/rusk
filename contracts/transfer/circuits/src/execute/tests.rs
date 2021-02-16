// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::test_helpers;

#[test]
fn execute_1_0() {
    test_helpers::execute_circuit::<15>(1, 0, true);
}

#[test]
fn execute_1_1() {
    test_helpers::execute_circuit::<15>(1, 1, true);
}

#[test]
fn execute_1_2() {
    test_helpers::execute_circuit::<15>(1, 2, true);
}

#[test]
fn execute_2_0() {
    test_helpers::execute_circuit::<16>(2, 0, true);
}

#[test]
fn execute_2_1() {
    test_helpers::execute_circuit::<16>(2, 1, true);
}

#[test]
fn execute_2_2() {
    test_helpers::execute_circuit::<16>(2, 2, true);
}

#[test]
fn execute_2_2_without_crossover() {
    test_helpers::execute_circuit::<16>(2, 2, false);
}

#[test]
fn execute_3_0() {
    test_helpers::execute_circuit::<17>(3, 0, true);
}

#[test]
fn execute_3_1() {
    test_helpers::execute_circuit::<17>(3, 1, true);
}

#[test]
fn execute_3_2() {
    test_helpers::execute_circuit::<17>(3, 2, true);
}

#[test]
fn execute_4_0() {
    test_helpers::execute_circuit::<17>(4, 0, true);
}

#[test]
fn execute_4_1() {
    test_helpers::execute_circuit::<17>(4, 1, true);
}

#[test]
fn execute_4_2() {
    test_helpers::execute_circuit::<17>(4, 2, true);
}
