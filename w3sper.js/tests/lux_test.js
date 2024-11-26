// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import { lux } from "@dusk/w3sper";

import { test, assert } from "./harness.js";

test("format Lux to Dusk", () => {
  assert.equal(lux.formatToDusk(BigInt(1e9)), "1");
  assert.equal(lux.formatToDusk(123_456_789_989n), "123.456789989");
  assert.equal(lux.formatToDusk(1n), "0.000000001");
  assert.equal(lux.formatToDusk(5889n), "0.000005889");
  assert.equal(
    lux.formatToDusk(1_000_999_973_939_759_000n),
    "1000999973.939759",
  );
  assert.equal(lux.formatToDusk(9_007_199_254_740_993n), "9007199.254740993");
  assert.equal(lux.formatToDusk(10_000_000_001n), "10.000000001");
  assert.equal(lux.formatToDusk(3_141_592_653_589_793n), "3141592.653589793");

  assert.equal(
    lux.formatToDusk(123_456_789_012_345_678_901_234_567_890n),
    "123456789012345678901.23456789",
  );

  assert.throws(
    () => lux.formatToDusk(10),
    TypeError,
    `Invalid Lux Type: Expected "bigint", but got "number"`,
  );

  assert.throws(
    () => lux.formatToDusk("10"),
    TypeError,
    `Invalid Lux Type: Expected "bigint", but got "string"`,
  );

  assert.throws(
    () => lux.formatToDusk(-11n),
    RangeError,
    `Lux values cannot be negative.`,
  );
});

test("parse Lux from Dusk", () => {
  assert.equal(lux.parseFromDusk("1"), BigInt(1e9));
  assert.equal(lux.parseFromDusk("123.456789989"), 123_456_789_989n);
  assert.equal(lux.parseFromDusk("0.000000001"), 1n);
  assert.equal(lux.parseFromDusk("0.000005889"), 5889n);
  assert.equal(
    lux.parseFromDusk("1000999973.939759"),
    1_000_999_973_939_759_000n,
  );
  assert.equal(lux.parseFromDusk("9007199.254740993"), 9_007_199_254_740_993n);
  assert.equal(lux.parseFromDusk("10.000000001"), 10_000_000_001n);
  assert.equal(lux.parseFromDusk("3141592.653589793"), 3_141_592_653_589_793n);

  assert.equal(
    lux.parseFromDusk("123456789012345678901.23456789"),
    123_456_789_012_345_678_901_234_567_890n,
  );

  assert.throws(
    () => lux.parseFromDusk("-10"),
    TypeError,
    `Cannot parse "-10" as a Lux value.`,
  );

  assert.throws(
    () => lux.parseFromDusk("1.0.1"),
    TypeError,
    `Cannot parse "1.0.1" as a Lux value.`,
  );
});
