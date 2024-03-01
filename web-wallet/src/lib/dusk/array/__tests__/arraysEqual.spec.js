import { describe, expect, it } from "vitest";
import arraysEqual from "../arraysEqual";

describe("arraysEqual", () => {
  it("returns false for arrays with different object references", () => {
    const obj1 = { key: "value" };
    const obj2 = { key: "value" };

    expect(arraysEqual([obj1], [obj2])).toBe(false);
  });

  it("distinguishes between 0 and -0", () => {
    expect(arraysEqual([0], [-0])).toBe(false);
  });

  it("considers NaN equal to NaN", () => {
    expect(arraysEqual([NaN], [NaN])).toBe(true);
  });

  it("returns true for arrays with the same elements in the same order", () => {
    expect(arraysEqual([1, 2, 3], [1, 2, 3])).toBe(true);
  });

  it("returns false for arrays with the same elements in different order", () => {
    expect(arraysEqual([1, 2, 3], [3, 2, 1])).toBe(false);
  });

  it("returns false for arrays with different elements", () => {
    expect(arraysEqual([1, 2, 3], [4, 5, 6])).toBe(false);
  });

  it("returns false for arrays of different lengths", () => {
    expect(arraysEqual([1, 2, 3], [1, 2, 3, 4])).toBe(false);
  });

  it("returns true when comparing an array with itself", () => {
    const array = [1, 2, 3];

    expect(arraysEqual(array, array)).toBe(true);
  });

  it("returns true for empty arrays", () => {
    expect(arraysEqual([], [])).toBe(true);
  });
});
