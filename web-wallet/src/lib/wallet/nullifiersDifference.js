import { mapWith } from "lamb";

const nullifiersToString = mapWith(String);

/**
 * Returns the array of unique nullifiers contained only
 * in the first of the two given nullifiers arrays.
 *
 * @see {@link https://en.wikipedia.org/wiki/Complement_(set_theory)#Relative_complement}
 *
 * @param {Uint8Array[]} a
 * @param {Uint8Array[]} b
 * @returns {Uint8Array[]}
 */
function nullifiersDifference(a, b) {
  if (a.length === 0 || b.length === 0) {
    return a;
  }

  const result = [];
  const lookup = new Set(nullifiersToString(b));

  for (const entry of a) {
    if (!lookup.has(entry.toString())) {
      result.push(entry);
    }
  }

  return result;
}

export default nullifiersDifference;
