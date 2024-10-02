import { describe, expect, it } from "vitest";
import { getKey, group, pluckFrom } from "lamb";

import { cacheUnspentNotes } from "$lib/mock-data";

import { notesArrayToMap } from "..";

describe("notesArrayToMap", () => {
  it("should convert an array of notes to the map format used by `w3sper.js`", () => {
    const notesMap = notesArrayToMap(cacheUnspentNotes);
    const groupedNotes = group(cacheUnspentNotes, getKey("address"));
    const addresses = Object.keys(groupedNotes);

    expect(Array.from(notesMap.keys())).toStrictEqual(addresses);

    addresses.forEach((address) => {
      const expectedKeys = pluckFrom(groupedNotes[address], "nullifier");
      const expectedValues = pluckFrom(groupedNotes[address], "note");
      const noteMap = notesMap.get(address) ?? new Map();

      expect(Array.from(noteMap.keys())).toStrictEqual(expectedKeys);
      expect(Array.from(noteMap.values())).toStrictEqual(expectedValues);
    });
  });
});
