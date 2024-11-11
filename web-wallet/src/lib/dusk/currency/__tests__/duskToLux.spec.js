import { describe, expect, it } from "vitest";

import { duskToLux } from "..";

describe("duskToLux", () => {
  it("should convert an amount in Dusk to Lux", () => {
    expect(duskToLux(1)).toBe(BigInt(1e9));
    expect(duskToLux(21.78)).toBe(21_780_000_000n);
    expect(duskToLux(3_456_789.012)).toBe(BigInt(3_456_789_012_000_000));

    // handles numbers in exponential notation
    expect(duskToLux(2e21)).toBe(BigInt(2e21) * BigInt(1e9));
  });
});
