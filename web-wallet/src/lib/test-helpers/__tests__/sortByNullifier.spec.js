import { describe, expect, it } from "vitest";

import { cacheSpentNotes } from "$lib/mock-data";

import { sortByNullifier } from "..";

describe("sortByNullifier", () => {
  it("should sort a list of notes by their nullifier", () => {
    const sortedNotes = cacheSpentNotes.toSorted((a, b) =>
      a.nullifier.toString() > b.nullifier.toString() ? 1 : -1
    );

    expect(sortByNullifier(cacheSpentNotes)).toStrictEqual(sortedNotes);
  });
});
