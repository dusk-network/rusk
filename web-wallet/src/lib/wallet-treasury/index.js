import { map } from "lamb";

import notesArrayToMap from "$lib/wallet/notesArrayToMap";
import walletCache from "$lib/wallet-cache";
import networkStore from "$lib/stores/networkStore";

class WalletTreasury {
  /** @type {AccountBalance[]} */
  #accountBalances = [];

  #profiles;

  /** @param {Array<import("$lib/vendor/w3sper.js/src/mod").Profile>} profiles */
  constructor(profiles = []) {
    this.#profiles = profiles;
  }

  /**
   * @param {import("$lib/vendor/w3sper.js/src/mod").Profile["address"]} identifier
   * @returns {Promise<AccountBalance>}
   */
  async account(identifier) {
    const balance = this.#accountBalances.at(+identifier);

    return (
      balance ??
      Promise.reject(
        new Error("No balance found for the account with the given identifier")
      )
    );
  }

  /**
   * @param {import("$lib/vendor/w3sper.js/src/mod").Profile["address"]} identifier
   * @returns {Promise<Map<Uint8Array, Uint8Array>>}
   */
  async address(identifier) {
    const address = identifier.toString();
    const result = [];
    const notes = await walletCache.getUnspentNotes([address]);

    for (const note of notes) {
      if (
        (await walletCache.getPendingNotesInfo([note.nullifier])).length === 0
      ) {
        result.push(note);
      }
    }

    return result.length
      ? /** @type {Map<Uint8Array, Uint8Array>} */ (
          notesArrayToMap(result).get(address)
        )
      : new Map();
  }

  /** @param {Array<import("$lib/vendor/w3sper.js/src/mod").Profile>} profiles */
  setProfiles(profiles) {
    this.#profiles = profiles;
  }

  /**
   * @param {bigint | import("$lib/vendor/w3sper.js/src/mod").Bookmark} from
   * @param {(evt: CustomEvent) => void} syncIterationListener
   * @param {AbortSignal} signal
   */
  // eslint-disable-next-line max-statements
  async update(from, syncIterationListener, signal) {
    const accountSyncer = await networkStore.getAccountSyncer();
    const addressSyncer = await networkStore.getAddressSyncer({ signal });

    // @ts-ignore
    addressSyncer.addEventListener("synciteration", syncIterationListener);

    this.#accountBalances = await accountSyncer.balances(this.#profiles);

    const notesStream = await addressSyncer.notes(this.#profiles, {
      from,
      signal,
    });

    for await (const [notesInfo, syncInfo] of notesStream) {
      await walletCache.addUnspentNotes(
        walletCache.toCacheNotes(notesInfo, this.#profiles),
        syncInfo
      );
    }

    // gather all unspent nullifiers in the cache
    const currentUnspentNullifiers =
      await walletCache.getUnspentNotesNullifiers();

    /**
     * Retrieving the nullifiers that are now spent.
     *
     * Currently `w3sper.js` returns an array of `ArrayBuffer`s
     * instead of one of `Uint8Array`s, but we don't
     * care as `ArrayBuffers` will be written in the
     * database anyway.
     */
    const spentNullifiers = await addressSyncer.spent(currentUnspentNullifiers);

    // update the cache with the spent nullifiers info
    await walletCache.spendNotes(spentNullifiers);

    // gather all spent nullifiers in the cache
    const currentSpentNullifiers =
      await walletCache.getUnspentNotesNullifiers();

    /**
     * Retrieving the nullifiers that are really spent given our
     * list of spent nullifiers.
     * We make this check because a block can be rejected and
     * we may end up having some notes marked as spent in the
     * cache, while they really aren't.
     *
     * Currently `w3sper.js` returns an array of `ArrayBuffer`s
     * instead of one of `Uint8Array`s.
     * @type {ArrayBuffer[]}
     */
    const reallySpentNullifiers = await addressSyncer.spent(
      currentSpentNullifiers
    );

    /**
     * As the previous `addressSyncer.spent` call returns a subset of
     * our spent nullifiers, we can skip this operation if the lengths
     * are the same.
     */
    if (reallySpentNullifiers.length !== currentSpentNullifiers.length) {
      const nullifiersToUnspend = walletCache.nullifiersDifference(
        currentSpentNullifiers,
        map(reallySpentNullifiers, (buf) => new Uint8Array(buf))
      );

      await walletCache.unspendNotes(nullifiersToUnspend);
    }

    // @ts-ignore
    addressSyncer.removeEventListener("synciteration", syncIterationListener);
  }
}

export default WalletTreasury;
