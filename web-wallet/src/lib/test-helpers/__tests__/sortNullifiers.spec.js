import { describe, expect, it } from "vitest";

import { cacheSpentNotes } from "$lib/mock-data";

import { sortNullifiers } from "..";

describe("sortNullifiers", () => {
  it("should sort a list of nullifiers", () => {
    const spentNotesNullifiers = cacheSpentNotes.map((n) => n.nullifier);
    const sortedNullifiers = spentNotesNullifiers.toSorted((a, b) =>
      String(a) > String(b) ? 1 : -1
    );

    expect(sortNullifiers(spentNotesNullifiers)).toStrictEqual(
      sortedNullifiers
    );
  });
});
