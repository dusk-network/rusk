import { map, setPathIn } from "lamb";

import notesArrayToMap from "$lib/wallet/notesArrayToMap";
import walletCache from "$lib/wallet-cache";
import networkStore from "$lib/stores/networkStore";

class WalletTreasury {
  /** @type {AccountBalance[]} */
  #accountBalances = [];

  #profiles;

  /** @type {StakeInfo[]} */
  #accountStakeInfo = [];

  /**
   * @param {bigint} lastBlockHeight
   * @returns {Promise<WalletCacheSyncInfo>}
   */
  async #getEnrichedSyncInfo(lastBlockHeight) {
    const [currentSyncInfo, lastBlockHash, lastFinalizedBlockHeight] =
      await Promise.all([
        walletCache.getSyncInfo(),
        networkStore.getBlockHashByHeight(lastBlockHeight).catch(() => ""),
        networkStore.getLastFinalizedBlockHeight().catch(() => 0n),
      ]);

    return {
      block: {
        hash: lastBlockHash,
        height: lastBlockHeight,
      },
      bookmark: currentSyncInfo.bookmark,
      lastFinalizedBlockHeight,
    };
  }

  /** @param {Profile[]} profiles */
  constructor(profiles = []) {
    this.#profiles = profiles;
  }

  /**
   * @param {Profile["account"]} identifier
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
   * @param {Profile["address"]} identifier
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

  /** @returns {Promise<void>} */
  async clearCache() {
    await walletCache.clear();
  }

  /**
   * @param {Profile} profile
   * @returns {Promise<WalletCacheBalanceInfo["balance"]>}
   */
  async getCachedBalance(profile) {
    return await walletCache.getBalanceInfo(profile.address.toString());
  }

  /**
   * @param {Profile} profile
   * @returns {Promise<StakeInfo>}
   */
  async getCachedStakeInfo(profile) {
    return await walletCache.getStakeInfo(profile.account.toString());
  }

  /**
   * @returns {Promise<WalletCacheSyncInfo>}
   */
  async getCachedSyncInfo() {
    return await walletCache.getSyncInfo();
  }

  /**
   * @param {Profile} profile
   * @param {WalletCacheBalanceInfo["balance"]} balance
   * @returns {Promise<void>}
   */
  async setCachedBalance(profile, balance) {
    await walletCache.setBalanceInfo(profile.address.toString(), balance);
  }

  /**
   * @param {Profile} profile
   * @param {StakeInfo} stakeInfo
   * @returns {Promise<void>}
   */
  async setCachedStakeInfo(profile, stakeInfo) {
    await walletCache.setStakeInfo(profile.account.toString(), stakeInfo);
  }

  /** @param {Profile[]} profiles */
  setProfiles(profiles) {
    this.#profiles = profiles;
  }

  /**
   * @param {Profile["account"]} identifier
   * @returns {Promise<StakeInfo>}
   */
  async stakeInfo(identifier) {
    const stakeInfo = this.#accountStakeInfo.at(+identifier);

    return (
      stakeInfo ??
      Promise.reject(
        new Error(
          "No stake info found for the account with the given identifier"
        )
      )
    );
  }

  /**
   * @param {bigint | Bookmark} from
   * @param {(evt: CustomEvent) => void} syncIterationListener
   * @param {AbortSignal} signal
   */
  // eslint-disable-next-line max-statements
  async update(from, syncIterationListener, signal) {
    let lastBlockHeight = 0n;

    /** @type {(evt: CustomEvent) => void} */
    const lastBlockHeightListener = ({ detail }) => {
      lastBlockHeight = detail.blocks.last;
    };
    const accountSyncer = await networkStore.getAccountSyncer();
    const addressSyncer = await networkStore.getAddressSyncer();

    // @ts-ignore
    addressSyncer.addEventListener("synciteration", lastBlockHeightListener);

    // @ts-ignore
    addressSyncer.addEventListener("synciteration", syncIterationListener);

    [this.#accountBalances, this.#accountStakeInfo] = await Promise.all([
      accountSyncer.balances(this.#profiles),
      accountSyncer.stakes(this.#profiles),
    ]);

    const notesStream = await addressSyncer.notes(this.#profiles, {
      from,
      signal,
    });

    /**
     * For each chunk of data in the stream we enrich the sync
     * info with the block hash, that will be used to check that
     * our local state is consistent with the remote one.
     * This way we can ensure that if a user interrupts the sync
     * while it's still in progress we can safely resume it from
     * the stored bookmark if no block has been rejected in the
     * meantime.
     */
    for await (const [notesInfo, streamSyncInfo] of notesStream) {
      const notesSyncInfo = {
        block: {
          hash: await networkStore
            .getBlockHashByHeight(streamSyncInfo.blockHeight)
            .catch(() => ""),
          height: streamSyncInfo.blockHeight,
        },
        bookmark: streamSyncInfo.bookmark,
      };
      await walletCache.addUnspentNotes(
        walletCache.toCacheNotes(notesInfo, this.#profiles),
        notesSyncInfo
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

    /**
     * We enrich the sync info by retrieving the hash of the last
     * processed block and the height of the last finalized block.
     * We'll use this information at the start of the sync
     * to determine if a block has been rejected, so that we can
     * fix our local cache state by syncing from the last finalized
     * block height.
     */
    await walletCache.setSyncInfo(
      await this.#getEnrichedSyncInfo(lastBlockHeight)
    );

    // @ts-ignore
    addressSyncer.removeEventListener("synciteration", lastBlockHeightListener);

    // @ts-ignore
    addressSyncer.removeEventListener("synciteration", syncIterationListener);
  }

  /**
   * @param {Profile} profile
   * @param {bigint} nonce
   * @returns {Promise<void>}
   */
  async updateCachedNonce(profile, nonce) {
    const address = profile.address.toString();
    const currentBalance = await walletCache.getBalanceInfo(address);

    await walletCache.setBalanceInfo(
      address,
      setPathIn(currentBalance, "unshielded.nonce", nonce)
    );
  }

  /**
   * @param {Uint8Array[]} nullifiers
   * @param {string} hash
   * @returns {Promise<void>}
   */
  async updateCachedPendingNotes(nullifiers, hash) {
    await walletCache.setPendingNotesInfo(nullifiers, hash);
  }
}

export default WalletTreasury;
