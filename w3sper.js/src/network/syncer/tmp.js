const HISTORY_CHUNK_SIZE = 200n;

const max = (a, b) => (a > b ? a : b);
const min = (a, b) => (a < b ? a : b);

const isHistoryRangeValid = ({ order, from, to }, range) =>
  order === "asc" ? range.from < to : range.to > from;

export class AccountSyncer extends EventTarget {
  /** @type {Object} */
  #network;

  /**
   * Creates an AccountSyncer instance.
   * @param {Object} network - The network interface for accessing accounts.
   */
  constructor(network) {
    super();
    this.#network = network;
  }

  #createHistoryStream(profile, options) {
    const { order, from, limit, signal, to } = options;
    const key = profile.account.toString();

    let nextRange =
      order === "asc"
        ? {
            from,
            to: min(from + HISTORY_CHUNK_SIZE, to),
          }
        : {
            from: max(to - HISTORY_CHUNK_SIZE, from),
            to,
          };

    let enqueued = 0;

    return new ReadableStream({
      cancel(reason) {
        console.log(`Account history stream canceled (${key}):`, reason);
      },

      pull: async (controller) => {
        if (signal?.aborted) {
          this.cancel(signal.reason ?? "Abort signal received");
          controller.close();
          return;
        }

        let entries = [];

        while (
          entries.length === 0 &&
          isHistoryRangeValid(options, nextRange)
        ) {
          entries = await this.#network
            .query(
              `fullMoonlightHistory(
             address: "${key}",
             fromBlock: ${nextRange.from},
             ord: "${order}",
             toBlock: ${nextRange.to}
           ) { json }`,
              { signal }
            )
            .then((result) => result.fullMoonlightHistory?.json ?? [])
            .then((moonlightHistory) =>
              Promise.all(
                moonlightHistory.map((historyEntry) =>
                  this.#network
                    .query(
                      `tx(hash: "${historyEntry.origin}") {
                      blockHash,
                      blockTimestamp,
                      err,
                      gasSpent,
                      tx {
                        callData {
                          fnName
                        },
                        gasLimit,
                        gasPrice,
                        isDeploy,
                        memo
                      }
                    }`
                    )
                    .then(({ tx }) =>
                      toW3sperHistoryEntry(key, historyEntry, tx)
                    )
                )
              )
            )
            .catch((error) => {
              console.error(`Error fetching account history (${key})`, error);
              controller.error(error);
            });

          if (order === "asc") {
            nextRange.from = nextRange.to + 1n;
            nextRange.to = min(nextRange.from + HISTORY_CHUNK_SIZE, to);
          } else {
            nextRange.to = nextRange.from - 1n;
            nextRange.from = max(nextRange.to - HISTORY_CHUNK_SIZE, from);
          }
        }

        for (let i = 0; i < entries.length && enqueued < limit; i++) {
          controller.enqueue(entries[i]);
          enqueued++;
        }

        if (enqueued >= limit || !isHistoryRangeValid(options, nextRange)) {
          controller.close();
        }
      },
    });
  }

  /**
   * Fetches the moonlight transactions history for
   * the given profiles.
   *
   * @param {Array<Object>} profiles
   * @param {Object} [options={}]
   * @param {bigint} [options.from]
   * @param {number} [options.limit] Max entries per profile
   * @param {string} [options.order="asc"] "asc" or "desc"
   * @param {AbortSignal} [options.signal]
   * @param {bigint} [options.to] Defaults to current block height
   * @returns {Promise<ReadableStream[]>}
   */
  async history(profiles, options = {}) {
    options = {
      from: options.from ?? 0n,
      limit: options.limit ?? Infinity,
      order: options.order === "asc" ? "asc" : "desc",
      signal: options.signal,
      to: options.to ?? (await this.#network.blockHeight),
    };

    return profiles.map((profile) =>
      this.#createHistoryStream(profile, options)
    );
  }
}
