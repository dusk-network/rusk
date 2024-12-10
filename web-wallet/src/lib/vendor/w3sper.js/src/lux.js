// @ts-nocheck
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

/**
 * A module for converting between the `BigInt` representation of Lux values
 * and the human-readable Dusk format.
 *
 * The Dusk format represents Lux values as floating-point numbers with a fixed
 * precision of 9 decimal places. This module ensures precise conversions and
 * validates inputs for consistency and correctness.
 *
 * ## Notes on the Lux Representation
 *
 * Lux values are stored as unsigned 64-bit integers (`u64` in our contracts),
 * meaning they cannot represent negative numbers. This restriction is inherent
 * to their data type. Any attempt to format or parse negative numbers would
 * violate this constraint and result in invalid operations.
 *
 * The unsigned 64-bit format also provides a higher maximum value
 * (`18446744073709551615`) compared to signed 64-bit integers, which are
 * capped at `9223372036854775807`. By restricting values to non-negative
 *  numbers, the module adheres to the `u64` representation, ensuring accurate
 * handling and avoiding unexpected behaviors or loss of precision.
 *
 * @module lux.js
 */
const DECIMALS = 9;
const SCALE_FACTOR = 10n ** BigInt(DECIMALS);
const FLOATING_POINT_REGEX = /^\d+(\.\d+)?$/;

/**
 * Converts a `BigInt` representation of a Lux value into a human-readable
 * string formatted in Dusk.
 *
 * The resulting string represents the Lux value as a floating-point number
 * with up to `DECIMALS` decimal places, omitting trailing zeros.
 *
 * @param {bigint} lux - The `BigInt` representation of the Lux value.
 *
 * @throws {TypeError} If the input is not a `BigInt`.
 * @throws {RangeError} If the input is a negative value.

 * @returns {string} A formatted string representing the Lux value in Dusk.
 *
 * @example usage
 * ```js
 * formatToDusk(11000000001n); // "11.000000001"
 * formatToDusk(11000000000n); // "11"
 *```
 */
export function formatToDusk(lux) {
  if (typeof lux !== "bigint") {
    throw new TypeError(
      `Invalid Lux Type: Expected "bigint", but got "${typeof lux}".`
    );
  }

  if (lux < 0n) {
    throw new RangeError("Lux values cannot be negative.");
  }

  const unit = lux / SCALE_FACTOR;
  let decimals = lux % SCALE_FACTOR;

  if (decimals > 0) {
    return `${unit}.${decimals.toString().padStart(DECIMALS, "0").replace(/0+$/, "")}`;
  }

  return unit.toString();
}

/**
 * Parses a Dusk string into a `BigInt` representation of the value in Lux.
 *
 * The Dusk string must be a valid positive floating-point number representation
 * (e.g., "11.000000001"), as Lux values are unsigned by definition.
 * Trailing zeros in the fractional part are allowed, and the string will be
 * interpreted with a fixed precision of `DECIMALS`.
 *
 * @param {string} dusk - The string formatted in Dusk to parse.
 *
 * @throws {TypeError} If the input is not a valid Dusk string.
 *
 * @returns {bigint} The `BigInt` representation of the Lux value.
 *
 * @example usage
 * ```js
 * parseFromDusk("11.000000001"); // 11000000001n
 * parseFromDusk("11"); // 11000000000n
 * ```
 */
export function parseFromDusk(dusk) {
  if (typeof dusk !== "string" || !FLOATING_POINT_REGEX.test(dusk)) {
    throw new TypeError(`Cannot parse "${dusk}" as a Lux value.`);
  }

  const [units, decimals = "0"] = dusk.split(".");

  return BigInt(units) * SCALE_FACTOR + BigInt(decimals.padEnd(DECIMALS, "0"));
}
