import { describe, expect, it } from "vitest";

import { luxToDusk } from "..";

describe("luxToDusk", () => {
  it("should take a lux value, as a bigInt, and return Dusk (as a number)", () => {
    expect(luxToDusk(BigInt(1e9))).toBe(1);
    expect(luxToDusk(123_456_789_012n)).toBe(123.456789012);
  });
});
