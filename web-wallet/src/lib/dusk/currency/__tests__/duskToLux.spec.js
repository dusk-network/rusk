import { describe, expect, it } from "vitest";

import { duskToLux } from "..";

describe("duskToLux", () => {
  it("should convert an amount in Dusk to Lux", () => {
    expect(duskToLux(1)).toBe(BigInt(1e9));
    expect(duskToLux(21.78)).toBe(21_780_000_000n);
    expect(duskToLux(3_456_789.012)).toBe(3_456_789_012_000_000n);

    // handles numbers in exponential notation
    expect(duskToLux(2e21)).toBe(BigInt(2e21) * BigInt(1e9));

    // we would lose 1 Lux in these numbers without rounding
    // the decimal part in the conversion function
    expect(duskToLux(1_000_999.973939759)).toBe(1_000_999_973_939_759n);
    expect(duskToLux(45.123456999)).toBe(45_123_456_999n);
  });
});
