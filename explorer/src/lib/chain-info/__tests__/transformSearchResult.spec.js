import { describe, expect, it } from "vitest";

import { gqlSearchPossibleResults } from "$lib/mock-data";

import { transformSearchResult } from "..";

describe("transformSearchResult", () => {
  it("should transform an array of search results from the GraphQL API into an array of application search results", () => {
    /**
     * Some of the input data has both a block and a transaction as a result,
     * which is not a possible outcome right now, but the transform function
     * is able to handle the case.
     */
    expect(transformSearchResult(gqlSearchPossibleResults)).toStrictEqual([
      {
        id: "fda46b4e06cc78542db9c780adbaee83a27fdf917de653e8ac34294cf924dd63",
        type: "block",
      },
      {
        id: "fda46b4e06cc78542db9c780adbaee83a27fdf917de653e8ac34294cf924dd63",
        type: "block",
      },
      {
        id: "fda46b4e06cc78542db9c780adbaee83a27fdf917de653e8ac34294cf924dd63",
        type: "block",
      },
      {
        id: "38a21c90324dd1ea8a3eb749a520d4f33b2304fe150eb121086fb4cf13777908",
        type: "transaction",
      },
      {
        id: "38a21c90324dd1ea8a3eb749a520d4f33b2304fe150eb121086fb4cf13777908",
        type: "transaction",
      },
    ]);
  });

  it("should return an empty array if there is no data in the source results", () => {
    expect(
      transformSearchResult([
        {
          block: null,
          transaction: null,
        },
        {
          block: null,
        },
      ])
    ).toStrictEqual([]);
  });

  it("should return an empty array if it receives an empty array", () => {
    expect(transformSearchResult([])).toStrictEqual([]);
  });
});
