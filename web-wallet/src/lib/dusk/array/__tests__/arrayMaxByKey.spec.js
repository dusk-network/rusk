import { describe, expect, it } from "vitest";

import { arrayMaxByKey } from "..";

describe("arrayMaxByKey", () => {
  const testArr = [
    { uid: "id1", value: 5 },
    { uid: "id2", value: 3 },
    { uid: "id3", value: 6 },
  ];

  it("should get the max numeric value of a key in an array of objects", () => {
    expect(arrayMaxByKey("value")(testArr)).toBe(6);
  });
});
