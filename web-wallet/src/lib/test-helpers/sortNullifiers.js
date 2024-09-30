import { sortWith } from "lamb";

/**
 * We need to sort the nullifiers in tests as the
 * database doesn't guarantee a sort order.
 *
 * @type {(entries: Uint8Array[]) => Uint8Array[]}
 */
const sortNullifiers = sortWith([String]);

export default sortNullifiers;
