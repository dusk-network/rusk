// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use crate::test_helpers;

#[test]
fn execute_1_0() {
    test_helpers::execute_circuit(1, 0, true);
}

#[test]
fn execute_1_1() {
    test_helpers::execute_circuit(1, 1, true);
}

#[test]
fn execute_1_2() {
    test_helpers::execute_circuit(1, 2, true);
}

#[test]
fn execute_2_0() {
    test_helpers::execute_circuit(2, 0, true);
}

#[test]
fn execute_2_1() {
    test_helpers::execute_circuit(2, 1, true);
}

#[test]
fn execute_2_2() {
    test_helpers::execute_circuit(2, 2, true);
}

#[test]
fn execute_2_2_without_crossover() {
    test_helpers::execute_circuit(2, 2, false);
}

#[test]
fn execute_3_0() {
    test_helpers::execute_circuit(3, 0, true);
}

#[test]
fn execute_3_1() {
    test_helpers::execute_circuit(3, 1, true);
}

#[test]
fn execute_3_2() {
    test_helpers::execute_circuit(3, 2, true);
}

#[test]
fn execute_4_0() {
    test_helpers::execute_circuit(4, 0, true);
}

#[test]
fn execute_4_1() {
    test_helpers::execute_circuit(4, 1, true);
}

#[test]
fn execute_4_2() {
    test_helpers::execute_circuit(4, 2, true);
}
