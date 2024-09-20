import { sortWith } from "lamb";

/**
 * We need to sort the notes in tests as the
 * database doesn't guarantee a sort order.
 *
 * @type {(notes: WalletCacheNote[]) => WalletCacheNote[]}
 */
const sortCacheNotes = sortWith([
  /** @type {(entry: WalletCacheNote) => string} */ (
    (entry) => entry.nullifier.toString()
  ),
]);

export default sortCacheNotes;
