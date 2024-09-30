import { sortWith } from "lamb";

/**
 * We need to sort the entries in tests as the
 * database doesn't guarantee a sort order.
 *
 * @typedef {{ nullifier: Uint8Array }} T
 * @type {<U extends T>(entries: U[]) => U[]}
 */
const sortByNullifier = sortWith([
  /** @type {(entry: T) => string} */ ((entry) => entry.nullifier.toString()),
]);

export default sortByNullifier;
