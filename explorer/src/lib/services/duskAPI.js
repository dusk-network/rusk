import {
  apply,
  flatMapWith,
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
import { makeApiUrl, makeNodeUrl } from "$lib/url";

import {
  addCountAndUnique,
  calculateStats,
  transformBlock,
  transformSearchResult,
  transformTransaction,
} from "$lib/chain-info";

import * as gqlQueries from "./gql-queries";

/** @type {(blocks: GQLBlock[]) => Block[]} */
const transformBlocks = mapWith(transformBlock);

/** @type {(transactions: GQLTransaction[]) => Transaction[]} */
const transformTransactions = mapWith(transformTransaction);

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
 * @param {{ query: string, variables?: Record<string, string | number> }} queryInfo
 */
const gqlGet = (queryInfo) =>
  fetch(makeNodeUrl("/02/Chain"), {
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
 * @param {"alive_nodes" | "provisioners"} topic
 * @param {any} data
 */
const hostGet = (topic, data) =>
  fetch(makeNodeUrl("/2/rusk"), {
    body: JSON.stringify({ data, topic }),
    headers: {
      Accept: "application/json",
      "Accept-Charset": "utf-8",
      "Content-Type": "application/json",
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
  fetch(makeApiUrl(endpoint, params), {
    headers: {
      Accept: "application/json",
      "Accept-Charset": "utf-8",
    },
    method: "GET",
  })
    .then(failureToRejection)
    .then((res) => res.json());

/**
 * @param {string} endpoint
 * @returns {Promise<any>}
 */
const nodePost = (endpoint) =>
  fetch(makeNodeUrl(endpoint), {
    headers: {
      Accept: "application/json",
      "Accept-Charset": "utf-8",
    },
    method: "POST",
  })
    .then(failureToRejection)
    .then((res) => res.json());

/** @type {() => Promise<HostProvisioner[]>} */
const getProvisioners = () => hostGet("provisioners", "");

/** @type {() => Promise<number>} */
const getLastHeight = () =>
  gqlGet({
    query: "query { block(height: -1) { header { height } } }",
  }).then(getPath("block.header.height"));

/** @type {() => Promise<Pick<GQLTransaction, "err">[]>} */
const getLast100BlocksTxs = () =>
  gqlGet({
    query: "query { blocks(last: 100) { transactions { err } } }",
  })
    .then(getKey("blocks"))
    .then(flatMapWith(getKey("transactions")));

const duskAPI = {
  /**
   * @param {string} id
   * @returns {Promise<Block>}
   */
  getBlock(id) {
    return gqlGet(gqlQueries.getBlockQueryInfo(id))
      .then(async ({ block }) =>
        setPathIn(
          block,
          "header.nextBlockHash",
          await duskAPI.getBlockHashByHeight(block.header.height + 1)
        )
      )
      .then(transformBlock);
  },

  /**
   * @param {string} id
   * @returns {Promise<string>}
   */
  getBlockDetails(id) {
    return gqlGet(gqlQueries.getBlockDetailsQueryInfo(id)).then(
      getPath("block.header.json")
    );
  },

  /**
   * @param {number} height
   * @returns {Promise<string>}
   */
  getBlockHashByHeight(height) {
    return gqlGet(gqlQueries.getBlockHashQueryInfo(height)).then(({ block }) =>
      block ? block.header.hash : ""
    );
  },

  /**
   * @param {number} amount
   * @returns {Promise<Block[]>}
   */
  getBlocks(amount) {
    return gqlGet(gqlQueries.getBlocksQueryInfo(amount))
      .then(getKey("blocks"))
      .then(transformBlocks);
  },

  /**
   * @param {number} amount
   * @returns {Promise<ChainInfo>}
   */
  getLatestChainInfo(amount) {
    return gqlGet(gqlQueries.getLatestChainQueryInfo(amount)).then(
      ({ blocks, transactions }) => ({
        blocks: transformBlocks(blocks),
        transactions: transformTransactions(transactions),
      })
    );
  },

  /** @returns {Promise<MarketData>} */
  getMarketData() {
    /* eslint-disable camelcase */

    return apiGet("https://api.coingecko.com/api/v3/coins/dusk-network", {
      community_data: false,
      developer_data: false,
      localization: false,
      market_data: true,
      sparkline: false,
      tickers: false,
    })
      .then(getKey("market_data"))
      .then((data) => ({
        currentPrice: data.current_price,
        marketCap: data.market_cap,
      }));

    /* eslint-enable camelcase */
  },

  /**
   * @returns {Promise<NodeInfo>}
   */
  getNodeInfo() {
    return nodePost("/on/node/info").then((res) => {
      return res;
    });
  },

  /**
   * @returns {Promise<{ lat: number, lon: number}[]>}
   */
  getNodeLocations() {
    return nodePost("/on/network/peers_location").then((data) =>
      addCountAndUnique(data)
    );
  },

  /**
   * @returns {Promise<Stats>}
   */
  getStats() {
    return Promise.all([
      getProvisioners(),
      getLastHeight(),
      getLast100BlocksTxs(),
    ]).then(apply(calculateStats));
  },

  /**
   * @param {string} id
   * @returns {Promise<Transaction>}
   */
  getTransaction(id) {
    return gqlGet(gqlQueries.getTransactionQueryInfo(id))
      .then(getKey("tx"))
      .then(transformTransaction);
  },

  /**
   * @param {string} id
   * @returns {Promise<string>}
   */
  getTransactionDetails(id) {
    return gqlGet(gqlQueries.getTransactionDetailsQueryInfo(id)).then(
      getPath("tx.tx.json")
    );
  },

  /**
   * @param {number} amount
   * @returns {Promise<Transaction[]>}
   */
  getTransactions(amount) {
    return gqlGet(gqlQueries.getTransactionsQueryInfo(amount))
      .then(getKey("transactions"))
      .then(transformTransactions);
  },

  /**
   * @param {string} query
   * @returns {Promise<SearchResult[]>}
   */
  search(query) {
    return Promise.all(
      [
        query.length === 64
          ? gqlGet(gqlQueries.searchByHashQueryInfo(query))
          : undefined,
        /^\d+$/.test(query)
          ? gqlGet(gqlQueries.getBlockHashQueryInfo(+query))
          : undefined,
      ].filter(Boolean)
    ).then(transformSearchResult);
  },
};

export default duskAPI;
