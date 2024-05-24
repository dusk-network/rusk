import {
  fromPairs,
  getKey,
  getPath,
  isUndefined,
  mapWith,
  ownPairs,
  pipe,
  setPathIn,
  unless,
  updateAt,
} from "lamb";

import { failureToRejection } from "$lib/dusk/http";

import {
  transformAPIBlock,
  transformAPITransaction,
  transformBlock,
  transformSearchResult,
  transformTransaction,
} from "$lib/chain-info";

import * as gqlQueries from "./gql-queries";

/** @type {(blocks: APIBlock[]) => Block[]} */
const transformAPIBlocks = mapWith(transformAPIBlock);

/** @type {(transactions: APITransaction[]) => Transaction[]} */
const transformTransactions = mapWith(transformAPITransaction);

/** @type {(s: string) => string} */
const ensureTrailingSlash = (s) => (s.endsWith("/") ? s : `${s}/`);

/**
 * Adds the `Rusk-gqlvar-` prefix to all
 * keys of the given object.
 * Returns `undefined` if the input is `undefined`.
 */
const toHeadersVariables = unless(
  isUndefined,
  pipe([ownPairs, mapWith(updateAt(0, (k) => `Rusk-gqlvar-${k}`)), fromPairs])
);

/**
 * @param {string} endpoint
 * @param {Record<string, any> | undefined} params
 * @returns {URL}
 */
const makeAPIURL = (endpoint, params) =>
  new URL(
    `${endpoint}?${new URLSearchParams(params)}`,
    ensureTrailingSlash(import.meta.env.VITE_API_ENDPOINT)
  );

/**
 * @param {string} node
 * @param {{ query: string, variables: Record<string, string | number> | undefined }} queryInfo
 */
const gqlGet = (node, queryInfo) =>
  fetch(`https://${node}/02/Chain`, {
    body: JSON.stringify({
      data: queryInfo.query,
      topic: "gql",
    }),
    headers: {
      Accept: "application/json",
      "Accept-Charset": "utf-8",
      "Content-Type": "application/json",
      ...toHeadersVariables(queryInfo.variables),
    },
    method: "POST",
  })
    .then(failureToRejection)
    .then((res) => res.json());

/**
 * @param {string} endpoint
 * @param {Record<string, any>} [params]
 * @returns {Promise<any>}
 */
const apiGet = (endpoint, params) =>
  fetch(makeAPIURL(endpoint, params), {
    headers: {
      Accept: "application/json",
      "Accept-Charset": "utf-8",
    },
    method: "GET",
  })
    .then(failureToRejection)
    .then((res) => res.json());

const duskAPI = {
  /**
   * @param {string} node
   * @param {string} id
   * @returns {Promise<Block>}
   */
  getBlock(node, id) {
    return gqlGet(node, gqlQueries.getBlockQueryInfo(id))
      .then(async ({ block }) =>
        setPathIn(
          block,
          "header.nextBlockHash",
          await duskAPI.getBlockHashByHeight(node, block.header.height + 1)
        )
      )
      .then(transformBlock);
  },

  /**
   * @param {string} node
   * @param {number} height
   * @returns {Promise<string>}
   */
  getBlockHashByHeight(node, height) {
    return gqlGet(node, gqlQueries.getBlockHashQueryInfo(height)).then(
      ({ block }) => (block ? block.header.hash : "")
    );
  },

  /**
   * @param {string} node
   * @returns {Promise<Block[]>}
   */
  getBlocks(node) {
    return apiGet("blocks", { node })
      .then(getPath("data.blocks"))
      .then(transformAPIBlocks);
  },

  /**
   * @param {string} node
   * @returns {Promise<ChainInfo>}
   */
  getLatestChainInfo(node) {
    return apiGet("latest", { node })
      .then(getKey("data"))
      .then(({ blocks, transactions }) => ({
        blocks: transformAPIBlocks(blocks),
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
    return gqlGet(node, gqlQueries.getTransactionQueryInfo(id))
      .then(getKey("tx"))
      .then(transformTransaction);
  },

  /**
   * @param {string} node
   * @param {string} id
   * @returns {Promise<string>}
   */
  getTransactionDetails(node, id) {
    return gqlGet(node, gqlQueries.getTransactionDetailsQueryInfo(id)).then(
      getPath("tx.raw")
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

  /**
   * @param {string} node
   * @param {string} query
   * @returns {Promise<SearchResult[]>}
   */
  search(node, query) {
    return apiGet(`search/${encodeURIComponent(query)}`, { node }).then(
      transformSearchResult
    );
  },
};

export default duskAPI;
