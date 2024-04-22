import { getKey, getPath, mapWith } from "lamb";

import { failureToRejection } from "$lib/dusk/http";

import { transformBlock, transformTransaction } from "$lib/chain-info";

/** @type {(blocks: APIBlock[]) => Block[]} */
const transformBlocks = mapWith(transformBlock);

/** @type {(transactions: APITransaction[]) => Transaction[]} */
const transformTransactions = mapWith(transformTransaction);

/** @type {(s: string) => string} */
const ensureTrailingSlash = (s) => (s.endsWith("/") ? s : `${s}/`);

/**
 * @param {string} endpoint
 * @param {Record<string, any> | undefined} params
 * @returns {URL}
 */
const makeURL = (endpoint, params) =>
  new URL(
    `${endpoint}?${new URLSearchParams(params)}`,
    ensureTrailingSlash(import.meta.env.VITE_API_ENDPOINT)
  );

/**
 * @param {string} endpoint
 * @param {Record<string, any>} [params]
 * @returns {Promise<any>}
 */
const apiGet = (endpoint, params) =>
  fetch(makeURL(endpoint, params), {
    headers: {
      Accept: "application/json",
      "Accept-Charset": "utf-8",
    },
    method: "GET",
  })
    .then(failureToRejection)
    .then((res) => res.json());

export default {
  /**
   * @param {string} node
   * @param {string} id
   * @returns {Promise<Block>}
   */
  getBlock(node, id) {
    return apiGet(`blocks/${id}`, { node })
      .then(getPath("data.blocks.0"))
      .then(transformBlock);
  },

  /**
   * @param {string} node
   * @returns {Promise<Block[]>}
   */
  getBlocks(node) {
    return apiGet("blocks", { node })
      .then(getPath("data.blocks"))
      .then(transformBlocks);
  },

  /**
   * @param {string} node
   * @returns {Promise<ChainInfo>}
   */
  getLatestChainInfo(node) {
    return apiGet("latest", { node })
      .then(getKey("data"))
      .then(({ blocks, transactions }) => ({
        blocks: transformBlocks(blocks),
        transactions: transformTransactions(transactions),
      }));
  },

  /** @returns {Promise<MarketData>} */
  getMarketData() {
    return apiGet("quote")
      .then(getKey("market_data"))
      .then((data) => ({
        currentPrice: data.current_price,
        marketCap: data.market_cap,
      }));
  },

  /**
   * @param {string} node
   * @returns {Promise<{ lat: number, lon: number}[]>}
   */
  getNodeLocations(node) {
    return apiGet("locations", { node }).then(getKey("data"));
  },

  /**
   * @param {string} node
   * @returns {Promise<Stats>}
   */
  getStats(node) {
    return apiGet("stats", { node });
  },

  /**
   * @param {string} node
   * @param {string} id
   * @returns {Promise<Transaction>}
   */
  getTransaction(node, id) {
    return apiGet(`transactions/${id}`, { node })
      .then(getPath("data.0"))
      .then(transformTransaction);
  },

  /**
   * @param {string} node
   * @param {string} id
   * @returns {Promise<string>}
   */
  getTransactionDetails(node, id) {
    return apiGet(`transactions/${id}/details`, { node }).then(
      getPath("data.json")
    );
  },

  /**
   * @param {string} node
   * @returns {Promise<Transaction[]>}
   */
  getTransactions(node) {
    return apiGet("transactions", { node })
      .then(getKey("data"))
      .then(transformTransactions);
  },
};
