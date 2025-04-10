// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

import * as ProtocolDriver from "../../protocol-driver/mod.js";
import * as base58 from "../../encoders/b58.js";

const HISTORY_CHUNK_SIZE = 50n;

function parseEvents(account, historyEntryEvents) {
  const from = "N/A";
  const to = "N/A";
  const value = 0n;

  return { from, to, value };
}

function toW3sperHistoryEntry(account, historyEntry, gqlTransaction) {
  const { from, to, value } = parseEvents(account, historyEntry.events);
  const direction = account !== from ? "in" : from === to ? "self" : "out";

  return {
    blockHash: gqlTransaction.blockHash,
    blockHeight: BigInt(historyEntry.block_height),
    blockTimestamp: gqlTransaction.blockTimestamp * 1000,
    direction,
    feePaid: BigInt(gqlTransaction.gasSpent * gqlTransaction.tx.gasPrice),
    from,
    gasLimit: BigInt(gqlTransaction.tx.gasLimit),
    gasPrice: BigInt(gqlTransaction.tx.gasPrice),
    gasSpent: BigInt(gqlTransaction.gasSpent),
    hash: historyEntry.origin,
    memo: gqlTransaction.tx.memo ?? "",
    method: gqlTransaction.tx.isDeploy
      ? "deploy"
      : gqlTransaction.tx.callData?.fnName ?? "transfer",
    owner: account, // TODO ask Seppia if it's worth it
    success: gqlTransaction.err === null,
    to,
    type: "public",
    value,
  };
}

/**
 * Represents the value staked, locked, and eligibility of a stake.
 */
class StakeAmount {
  /** @type {bigint} */
  value = 0n;
  /** @type {bigint} */
  locked = 0n;
  /** @type {bigint} */
  eligibility = 0n;

  /**
   * Returns the total amount of staked value, including locked value.
   *
   * @returns {bigint} Total staked amount.
   */
  get total() {
    return this.value + this.locked;
  }
}

/**
 * Holds information about a user's stake, including amount, reward
 * and tracks faults.
 */
class StakeInfo {
  /** @type {StakeAmount|null} */
  amount;
  /** @type {bigint} */
  reward;
  /** @type {number} */
  faults;
  /** @type {number} */
  hardFaults;

  constructor() {
    this.amount = null;
    this.reward = 0n;
    this.faults = 0;
    this.hardFaults = 0;
  }

  /**
   * Parses a buffer into a {StakeInfo} instance.
   *
   * @param {ArrayBuffer} buffer - The buffer containing stake data.
   * @returns {StakeInfo} The parsed {StakeInfo} instance.
   */
  static parse(buffer) {
    const view = new DataView(buffer);
    const stakeInfo = new StakeInfo();
    const hasStake = view.getUint8(0) === 1;

    if (!hasStake) {
      return Object.freeze(stakeInfo);
    }

    const hasStakeAmount = view.getUint8(8) === 1;

    if (hasStakeAmount) {
      stakeInfo.amount = new StakeAmount();
      stakeInfo.amount.value = view.getBigUint64(16, true);
      stakeInfo.amount.locked = view.getBigUint64(24, true);
      stakeInfo.amount.eligibility = view.getBigUint64(32, true);
    }

    stakeInfo.reward = view.getBigUint64(40, true);
    stakeInfo.faults = view.getUint8(48);
    stakeInfo.hardFaults = view.getUint8(49);

    return Object.freeze(stakeInfo);
  }
}

/**
 * Converts a resource, either a string or an object with an account,
 * into an account buffer if it has a byteLength of 96.
 *
 * @param {Object|string} resource - The resource to convert.
 * @returns {ArrayBuffer|Object|string} The account buffer or the resource.
 */
function intoAccount(resource) {
  if (resource?.account?.valueOf()?.byteLength === 96) {
    return resource.account;
  } else if (typeof resource === "string") {
    const buffer = base58.decode(resource);
    if (buffer.byteLength === 96) {
      return buffer;
    }
  }

  return resource;
}

/**
 * Converts account profiles into raw representations.
 *
 * @param {Array<Object>} profiles - Array of profile objects.
 * @returns {Promise<Array<Uint8Array>>} The raw account buffers.
 */
const accountsIntoRaw = (profiles) =>
  ProtocolDriver.accountsIntoRaw(profiles.map(intoAccount));

/**
 * Parses a buffer to extract account balance information.
 *
 * @param {ArrayBuffer} buffer - The buffer containing balance data.
 * @returns {{ nonce: bigint, value: bigint }} The parsed balance data.
 */
const parseBalance = (buffer) => {
  const view = new DataView(buffer);
  const nonce = view.getBigUint64(0, true);
  const value = view.getBigUint64(8, true);

  return { nonce, value };
};

/**
 * Syncs account data by querying the network for balance and stake information.
 *
 * @extends EventTarget
 */
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
    const { order = "asc", from = 0n, signal, to } = options;
    const key = profile.account.toString();
    const nextRange = { from, to };

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

        const data = await this.#network
          .query(
            `fullMoonlightHistory(
             address: "${key}",
             fromBlock: ${nextRange.from},
             maxCount: ${HISTORY_CHUNK_SIZE},
             ord: "${order}",                      ,
             ${nextRange.to ? `toBlock: ${nextRange.to}` : ""}
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
                  .then(({ tx }) => toW3sperHistoryEntry(key, historyEntry, tx))
              )
            )
          )
          .catch((error) => {
            console.error(`Error fetching account history (${key})`, error);
            controller.error(error);
          });

        for (const entry of data) {
          controller.enqueue(entry);
        }

        if (data.length === 0) {
          controller.close();
        } else {
          if (order === "asc") {
            nextRange.from = data.at(-1).blockHeight + 1n;
          } else {
            nextRange.to = data.at(-1).blockHeight - 1n;
          }
        }
      },
    });
  }

  /**
   * Fetches the balances for the given profiles.
   *
   * @param {Array<Object>} profiles - Array of profile objects.
   * @returns {Promise<Array<{ nonce: bigint, value: bigint }>>} Array of balances.
   */
  async balances(profiles) {
    const balances = await accountsIntoRaw(profiles).then((rawUsers) =>
      rawUsers.map((user) =>
        this.#network.contracts.transferContract.call.account(user)
      )
    );

    return Promise.all(balances)
      .then((responses) => responses.map((resp) => resp.arrayBuffer()))
      .then((buffers) => Promise.all(buffers))
      .then((buffers) => buffers.map(parseBalance));
  }

  /**
   * Fetches the moonlight transactions history for
   * the given profiles.
   *
   * @param {Array<Object>} profiles
   * @param {Object} [options={}]
   * @param {bigint} [options.from]
   * @param {string} [options.order="asc"] "asc" or "desc"
   * @param {AbortSignal} [options.signal]
   * @param {bigint} [options.to]
   * @returns {ReadableStream[]}
   */
  history(profiles, options = {}) {
    return profiles.map((profile) =>
      this.#createHistoryStream(profile, options)
    );
  }

  /**
   * Fetches the stakes for the given profiles.
   *
   * @param {Array<Object>} profiles - Array of profile objects.
   * @returns {Promise<Array<StakeInfo>>} Array of parsed stake information.
   */
  async stakes(profiles) {
    const stakes = await accountsIntoRaw(profiles).then((rawUsers) =>
      rawUsers.map((user) =>
        this.#network.contracts.stakeContract.call.get_stake(user)
      )
    );

    return Promise.all(stakes)
      .then((responses) => responses.map((resp) => resp.arrayBuffer()))
      .then((buffers) => Promise.all(buffers))
      .then((buffers) => buffers.map(StakeInfo.parse));
  }
}
