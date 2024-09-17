import { describe, expect, it } from "vitest";

import { luxToDusk } from "..";

describe("luxToDusk", () => {
  it("should convert a number amount in Lux to Dusk", () => {
    expect(luxToDusk(1e9)).toBe(1);
    expect(luxToDusk(123_456_789_012)).toBe(123.456789012);
  });

  it("should accept `BigInt`s as input", () => {
    expect(luxToDusk(BigInt(1e9))).toBe(1);
    expect(luxToDusk(123_456_789_012n)).toBe(123.456789012);
  });
});
