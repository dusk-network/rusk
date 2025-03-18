import { describe, expect, it } from "vitest";

import { gqlSearchPossibleResults } from "$lib/mock-data";

import { transformSearchResult } from "..";

describe("transformSearchResult", () => {
  it("should transform an array of search results from the GraphQL API into an array of application search results", () => {
    expect(transformSearchResult(gqlSearchPossibleResults[0])).toStrictEqual({
      id: "fda46b4e06cc78542db9c780adbaee83a27fdf917de653e8ac34294cf924dd63",
      type: "block",
    });
    expect(transformSearchResult(gqlSearchPossibleResults[3])).toStrictEqual({
      id: "38a21c90324dd1ea8a3eb749a520d4f33b2304fe150eb121086fb4cf13777908",
      type: "transaction",
    });
  });

  it("should return null if there is no data in the source results", () => {
    expect(
      transformSearchResult({
        account: null,
        block: null,
        tx: null,
      })
    ).toStrictEqual(null);
  });
});
