import { describe, expect, it } from "vitest";

import { luxToDusk } from "..";

describe("luxToDusk", () => {
  it("should take a lux value, as a bigInt, and return Dusk (as a number)", () => {
    expect(luxToDusk(BigInt(1e9))).toBe(1);
    expect(luxToDusk(123_456_789_989n)).toBe(123.456789989);
    expect(luxToDusk(1n)).toBe(0.000000001);
    expect(luxToDusk(5889n)).toBe(0.000005889);
    expect(luxToDusk(1_000_999_973_939_759_000n)).toBe(1_000_999_973.939759);
    expect(luxToDusk(9_007_199_254_740_993n)).toBe(9_007_199.254740993);
    expect(luxToDusk(10_000_000_001n)).toBe(10.000000001);
    expect(luxToDusk(3_141_592_653_589_793n)).toBe(3_141_592.653589793);
  });
});
