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
} from "lamb";

import { failureToRejection } from "$lib/dusk/http";

import {
  transformBlock,
  transformSearchResult,
  transformTransaction,
} from "$lib/chain-info";

import * as gqlQueries from "./gql-queries";

/** @type {(blocks: GQLBlock[]) => Block[]} */
const transformBlocks = mapWith(transformBlock);

/** @type {(transactions: GQLTransaction[]) => Transaction[]} */
const transformTransactions = mapWith(transformTransaction);

/** @type {(s: string) => string} */
const ensureTrailingSlash = (s) => (s.endsWith("/") ? s : `${s}/`);

/**
 * Adds the `Rusk-gqlvar-` prefix to all
 * keys of the given object and calls `JSON.stringify`
 * on their values. *
 * Returns `undefined` if the input is `undefined`.
 *
 * The `JSON.stringify` call is because the GraphQL
 * server will parse a variable containing only digits
 * as a number otherwise, when the expected type is a string.
 */
const toHeadersVariables = unless(
  isUndefined,
  pipe([
    ownPairs,
    mapWith(([k, v]) => [`Rusk-gqlvar-${k}`, JSON.stringify(v)]),
    fromPairs,
  ])
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
   * @param {number} amount
   * @returns {Promise<Block[]>}
   */
  getBlocks(node, amount) {
    return gqlGet(node, gqlQueries.getBlocksQueryInfo(amount))
      .then(getKey("blocks"))
      .then(transformBlocks);
  },

  /**
   * @param {string} node
   * @param {number} amount
   * @returns {Promise<ChainInfo>}
   */
  getLatestChainInfo(node, amount) {
    return gqlGet(node, gqlQueries.getLatestChainQueryInfo(amount)).then(
      ({ blocks, transactions }) => ({
        blocks: transformBlocks(blocks),
        transactions: transformTransactions(transactions),
      })
    );
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
   * @param {number} amount
   * @returns {Promise<Transaction[]>}
   */
  getTransactions(node, amount) {
    return gqlGet(node, gqlQueries.getTransactionsQueryInfo(amount))
      .then(getKey("transactions"))
      .then(transformTransactions);
  },

  /**
   * @param {string} node
   * @param {string} query
   * @returns {Promise<SearchResult[]>}
   */
  search(node, query) {
    return Promise.all(
      [
        query.length === 64
          ? gqlGet(node, gqlQueries.searchByHashQueryInfo(query))
          : undefined,
        /^\d+$/.test(query)
          ? gqlGet(node, gqlQueries.getBlockHashQueryInfo(+query))
          : undefined,
      ].filter(Boolean)
    ).then(transformSearchResult);
  },
};

export default duskAPI;
