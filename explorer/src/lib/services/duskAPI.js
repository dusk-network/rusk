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
import * as base58 from "../utils/encoders/base58";

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
  fetch(makeNodeUrl("/on/graphql/query"), {
    body: queryInfo.query.replace(/\s+/g, " ").trim(),
    headers: {
      Accept: "application/json",
      "Accept-Charset": "utf-8",
      Connection: "Keep-Alive",
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
  fetch(makeApiUrl(endpoint, params), {
    headers: {
      Accept: "application/json",
      "Accept-Charset": "utf-8",
      Connection: "Keep-Alive",
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
      Connection: "Keep-Alive",
    },
    method: "POST",
  })
    .then(failureToRejection)
    .then((res) => res.json());

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

  /** @param {string} address */
  async getMoonlightAccountTransactions(address) {
    // Gets contract interactions for the given address
    const moonlightData = await gqlGet(
      gqlQueries.getFullMoonlightAccountHistoryQuery(address)
    );
    if (!moonlightData.fullMoonlightHistory) {
      return [];
    }
    // Extracts the transaction IDs from the contract interactions
    const transactionIds = moonlightData.fullMoonlightHistory.json.map(
      (/** @type {{ origin: any; }} */ block) => block.origin
    );
    // Fetches the transaction details for the extracted IDs
    const results = await Promise.all(
      transactionIds.map((/** @type {string} */ txnId) =>
        duskAPI.getTransaction(txnId)
      )
    );
    // Filters out transactions in the mempool
    const filteredTransactions = results.filter(
      (transaction) => typeof transaction !== "string"
    );
    // Sort transactions by date in descending order (newest first)
    const sortedTransactions = filteredTransactions.sort((a, b) => {
      const dateA = new Date(a.timestamp || a.time || a.date || 0);
      const dateB = new Date(b.timestamp || b.time || b.date || 0);
      return dateB.getTime() - dateA.getTime();
    });

    return sortedTransactions;
  },

  /**
   * @returns {Promise<NodeInfo>}
   */
  getNodeInfo() {
    return nodePost("/on/node/info");
  },

  /**
   * @returns {Promise<{ lat: number, lon: number}[]>}
   */
  getNodeLocations() {
    return nodePost("/on/network/peers_location").then((data) =>
      addCountAndUnique(data)
    );
  },

  /** @returns {Promise<HostProvisioner[]>} */
  getProvisioners() {
    return nodePost("/on/node/provisioners");
  },

  /**
   * @returns {Promise<Stats>}
   */
  getStats() {
    return Promise.all([
      duskAPI.getProvisioners(),
      getLastHeight(),
      getLast100BlocksTxs(),
    ]).then(apply(calculateStats));
  },

  /**
   * @param {string} id
   * @returns {Promise<Transaction | String>}
   */
  getTransaction(id) {
    return gqlGet(gqlQueries.getTransactionQueryInfo(id))
      .then(getKey("tx"))
      .then((tx) => {
        if (tx === null) {
          return gqlGet(gqlQueries.getMempoolTx(id))
            .then(getKey("mempoolTx"))
            .then((mempoolTx) => {
              if (mempoolTx) {
                return "This transaction is currently in the mempool and has not yet been confirmed. The transaction details will be displayed after confirmation.";
              } else {
                throw new Error("Transaction not found");
              }
            });
        } else {
          return transformTransaction(tx);
        }
      });
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
   * Search function that handles different query formats:
   * 1. 64-character hexadecimal strings (likely transaction or block hashes)
   * 2. Numeric strings (likely block heights)
   * 3. Base58-encoded strings with a decoded length of 96 bytes (likely addresses)
   *
   * @param {string} query - The search query string
   * @returns {Promise<SearchResult|null>} - Promise resolving to transformed search result
   */
  async search(query) {
    let searchPromise;

    // Case 1: Handle 64-character hexadecimal strings (likely tx or block hashes)
    if (query.length === 64) {
      searchPromise = gqlGet(gqlQueries.searchByHashQueryInfo(query));
    }

    // Case 2: Handle numeric strings (likely block heights)
    else if (/^\d+$/.test(query)) {
      searchPromise = gqlGet(gqlQueries.getBlockHashQueryInfo(+query));
    }

    // Case 3: Handle potential base58-encoded addresses
    else {
      const bytes = base58.decode(query);
      // If decoded length is 96 bytes, it's likely an address
      if (bytes?.length === 96) {
        searchPromise = Promise.resolve({ account: { id: query } });
      } else {
        searchPromise = Promise.resolve(null);
      }
    }

    const entry = await searchPromise;
    const result = entry ? transformSearchResult(entry) : null;
    return result;
  },
};

export default duskAPI;
