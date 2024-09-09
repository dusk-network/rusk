import { describe, expect, it } from "vitest";

import { arrayMaxByKey } from "..";

describe("arrayMaxByKey", () => {
  const testArr = [
    { active: true, date: new Date(2024, 1, 2), uid: "id3", value: 6, x: 0 },
    { active: false, date: new Date(2024, 1, 3), uid: "id2", value: 3, x: -0 },
    { active: false, date: new Date(2024, 1, 4), uid: "id1", value: 5, x: -1 },
  ];

  it("should get the max value of a key holding a comparable value in an array of objects", () => {
    expect(arrayMaxByKey("active")(testArr)).toBe(true);
    expect(arrayMaxByKey("date")(testArr)).toStrictEqual(new Date(2024, 1, 4));
    expect(arrayMaxByKey("uid")(testArr)).toBe("id3");
    expect(arrayMaxByKey("value")(testArr)).toBe(6);
  });

  it("will use the last encountered value if `0` or `-0` are the max value", () => {
    expect(arrayMaxByKey("x")(testArr)).toBe(-0);
  });

  it("should return `undefined` if given an empty array", () => {
    expect(arrayMaxByKey("value")([])).toBe(undefined);
  });
});
