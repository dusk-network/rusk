import { describe, expect, it } from "vitest";

import { arraySumByKey } from "..";

describe("arraySumByKey", () => {
  const testArr = [
    { uid: "id1", value: 5 },
    { uid: "id2", value: 3 },
    { uid: "id3", value: 6 },
  ];

  it("should sum the numeric values contained in a key in an array of objects", () => {
    expect(arraySumByKey("value")(testArr)).toBe(14);
  });

  it("should throw an exception if the build function receives an empty array", () => {
    expect(() => arraySumByKey("value")([])).toThrow(TypeError);
  });
});
