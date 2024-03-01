import { describe, expect, it } from "vitest";

import { lerp } from "..";

describe("lerp", () => {
  it("should perform a linear interpolation between the two given values", () => {
    expect(lerp(20, 80, 0.3)).toBe(38);
    expect(lerp(20, 80, 0.5)).toBe(50);
    expect(lerp(20, 80, 0.7)).toBe(62);
  });

  it("should return `a` with a normal value of `0`", () => {
    expect(lerp(20, 80, 0)).toBe(20);
  });

  it("should return `b` with a normal value of `1`", () => {
    expect(lerp(20, 80, 1)).toBe(80);
  });
});
