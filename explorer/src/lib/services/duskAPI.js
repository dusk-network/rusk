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
import { makeNodeUrl } from "$lib/url";

import {
  addCountAndUnique,
  calculateStats,
  transformBlock,
  transformSearchResult,
  transformTransaction,
} from "$lib/chain-info";

import {
  getBlockDetailsQueryInfo,
  getBlockHashQueryInfo,
  getBlockQueryInfo,
  getBlocksQueryInfo,
  getFullMoonlightAccountHistoryQuery,
  getLatestChainQueryInfo,
  getMempoolTx,
  getTransactionQueryInfo,
  getTransactionsQueryInfo,
  searchByHashQueryInfo,
  transactionFragment,
} from "./gql-queries";
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

const duskAPI = {
  /**
   * @param {string} address
   * @returns {Promise<AccountStatus>}
   */
  getAccountStatus(address) {
    return nodePost(`/on/account:${address}/status`);
  },

  /**
   * @param {string} id
   * @returns {Promise<Block>}
   */
  getBlock(id) {
    return gqlGet(getBlockQueryInfo(id))
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
    return gqlGet(getBlockDetailsQueryInfo(id)).then(
      getPath("block.header.json")
    );
  },

  /**
   * @param {number} height
   * @returns {Promise<string>}
   */
  getBlockHashByHeight(height) {
    return gqlGet(getBlockHashQueryInfo(height)).then(({ block }) =>
      block ? block.header.hash : ""
    );
  },

  /**
   * @param {number} amount
   * @returns {Promise<Block[]>}
   */
  getBlocks(amount) {
    return gqlGet(getBlocksQueryInfo(amount))
      .then(getKey("blocks"))
      .then(transformBlocks);
  },

  /**
   * @param {number} amount
   * @returns {Promise<ChainInfo>}
   */
  getLatestChainInfo(amount) {
    return gqlGet(getLatestChainQueryInfo(amount)).then(
      ({ blocks, transactions }) => ({
        blocks: transformBlocks(blocks),
        transactions: transformTransactions(transactions),
      })
    );
  },

  /** @returns {Promise<MarketData>} */
  async getMarketData() {
    const COINGECKO_MARKET_URL = new URL(
      "https://api.coingecko.com/api/v3/coins/dusk-network" +
        "?community_data=false" +
        "&developer_data=false" +
        "&localization=false" +
        "&market_data=true" +
        "&sparkline=false" +
        "&tickers=false"
    );

    try {
      // Fetch price data and circulating supply
      const [coinGeckoData, circulatingSupply] = await Promise.all([
        fetch(COINGECKO_MARKET_URL, {
          headers: {
            Accept: "application/json",
            "Accept-Charset": "utf-8",
            Connection: "Keep-Alive",
          },
          method: "GET",
        })
          .then(failureToRejection)
          .then((res) => res.json())
          .then(getKey("market_data")),
        fetch("https://supply.dusk.network/")
          .then(failureToRejection)
          .then((res) => res.text())
          .then((supply) => parseFloat(supply))
          .catch(() => null),
      ]);

      const currentPrice = coinGeckoData.current_price;

      // Calculate market cap using circulating supply if available, fallback to CoinGecko
      /** @type {Record<string, number>} */
      let marketCap;
      if (
        circulatingSupply !== null &&
        !isNaN(circulatingSupply) &&
        currentPrice?.usd
      ) {
        marketCap = {
          usd: circulatingSupply * currentPrice.usd,
        };
        // Add other currencies
        Object.keys(currentPrice).forEach((currency) => {
          if (currency !== "usd" && currentPrice[currency]) {
            marketCap[currency] = circulatingSupply * currentPrice[currency];
          }
        });
      } else {
        // Fallback to CoinGecko's market cap
        marketCap = coinGeckoData.market_cap;
      }

      return {
        currentPrice,
        marketCap,
      };
    } catch (/** @type {any} */ error) {
      throw new Error(`Failed to fetch market data: ${error.message}`);
    }
  },

  /**
   * @param {string} address
   * @returns {Promise<Transaction[]>} sortedTransactions
   */
  async getMoonlightAccountTransactions(address) {
    // Gets contract interactions for the given address
    const moonlightData = await gqlGet(
      getFullMoonlightAccountHistoryQuery(address)
    );
    if (!moonlightData.fullMoonlightHistory) {
      return [];
    }
    // Extracts the transaction IDs from the contract interactions
    const transactionIds = moonlightData.fullMoonlightHistory.json.map(
      (/** @type {{ origin: any; }} */ block) => block.origin
    );
    if (transactionIds.length === 0) return [];

    // Build a single GraphQL query with aliases for all txs
    /**
     * @param {string[]} ids
     * @returns {string}
     */
    const buildBatchTransactionQuery = (ids) => {
      const fragment = transactionFragment;
      const queries = ids.map(
        (id, idx) => `tx${idx}: tx(hash: "${id}") { ...TransactionInfo }`
      );
      return `query {\n${queries.join("\n")}\n}\n${fragment}`;
    };

    // Batch fetch all transactions in a single query
    const batchQuery = buildBatchTransactionQuery(transactionIds);
    const response = await gqlGet({ query: batchQuery });
    // response is an object: { tx0: {...}, tx1: {...}, ... }
    const txs = Object.values(response).filter(Boolean);
    const results = txs.map(transformTransaction);

    // Sort transactions by date in descending order (newest first)
    const sortedTransactions = results.sort((a, b) => {
      const dateA = a.date instanceof Date ? a.date.getTime() : 0;
      const dateB = b.date instanceof Date ? b.date.getTime() : 0;
      return dateB - dateA;
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
      duskAPI.getTxCount(),
    ]).then(([provisioners, lastHeight, txCount]) => ({
      ...calculateStats(provisioners, lastHeight),
      txCount,
    }));
  },

  /**
   * @param {string} id
   * @returns {Promise<Transaction | string>}
   */
  getTransaction(id) {
    return gqlGet(getTransactionQueryInfo(id))
      .then(getKey("tx"))
      .then((tx) => {
        if (tx === null) {
          return gqlGet(getMempoolTx(id))
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
   * @param {number} amount
   * @returns {Promise<Transaction[]>}
   */
  getTransactions(amount) {
    return gqlGet(getTransactionsQueryInfo(amount))
      .then(getKey("transactions"))
      .then(transformTransactions);
  },

  /**
   * @returns {Promise<{ public: number; shielded: number; total: number }>}
   */
  getTxCount() {
    return nodePost("/on/stats/tx_count");
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
      searchPromise = gqlGet(searchByHashQueryInfo(query));
    }

    // Case 2: Handle numeric strings (likely block heights)
    else if (/^\d+$/.test(query)) {
      searchPromise = gqlGet(getBlockHashQueryInfo(+query));
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
