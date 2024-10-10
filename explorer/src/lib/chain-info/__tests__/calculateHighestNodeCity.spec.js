import { describe, expect, it } from "vitest";

import { calculateHighestNodeCityCount } from "..";

describe("calculateHighestNodeCityCount", () => {
  it("should calculate the highest node count present in one city from an array of cities", () => {
    const mockData = [
      {
        city: "North Bergen",
        count: 15,
        country: "United States of America",
      },
      {
        city: "Clifton",
        count: 15,
        country: "United States of America",
      },
      {
        city: "North Bergen",
        count: 1,
        country: "United States of America",
      },
    ];
    const expectedResult = {
      city: "North Bergen",
      count: 16,
      country: "United States of America",
    };
    expect(calculateHighestNodeCityCount(mockData)).toStrictEqual(
      expectedResult
    );
  });
});
