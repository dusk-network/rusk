import { describe, expect, it } from "vitest";

import { luxToDusk } from "..";

describe("luxToDusk", () => {
  it("should convert an amount in Dusk to Lux", () => {
    expect(luxToDusk(1e9)).toBe(1);
    expect(luxToDusk(123_456_789_012)).toBe(123.456789012);
  });
});
