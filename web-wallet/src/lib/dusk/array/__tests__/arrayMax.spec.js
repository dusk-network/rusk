import { describe, expect, it } from "vitest";

import { arrayMax } from "..";

describe("arrayMax", () => {
  const testArr = [-0, 1, 3, 2, 4, 5];

  it("should get the max value in an array of numbers", () => {
    expect(arrayMax(testArr)).toBe(5);
    expect(arrayMax([])).toBe(-Infinity);
  });
});
