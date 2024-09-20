import { describe, expect, it } from "vitest";

import { cacheSpentNotes } from "$lib/mock-data";

import { sortCacheNotes } from "..";

describe("sortCacheNotes", () => {
  it("should sort a list of notes by their nullifier", () => {
    const sortedNotes = cacheSpentNotes.toSorted((a, b) =>
      a.nullifier.toString() > b.nullifier.toString() ? 1 : -1
    );

    expect(sortCacheNotes(cacheSpentNotes)).toStrictEqual(sortedNotes);
  });
});
