import { failureToRejection } from "$lib/dusk/http";

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
 * @returns {Promise<Response>}
 */
const apiGet = (endpoint, params) =>
  fetch(makeURL(endpoint, params), {
    headers: {
      Accept: "application/json",
      "Accept-Charset": "utf-8",
    },
    method: "GET",
  }).then(failureToRejection);

export default {
  /**
   * @param {string} node
   * @param {string} id
   */
  getBlock(node, id) {
    return apiGet(`blocks/${id}`, { node });
  },

  /** @param {string} node */
  getBlocks(node) {
    return apiGet("blocks", { node });
  },

  /** @param {string} node */
  getLatestChainInfo(node) {
    return apiGet("latest", { node });
  },

  getMarketData() {
    return apiGet("quote");
  },

  /** @param {string} node */
  getNodeLocations(node) {
    return apiGet("locations", { node });
  },

  /** @param {string} node */
  getStats(node) {
    return apiGet("stats", { node });
  },

  /**
   * @param {string} node
   * @param {string} id
   */
  getTransaction(node, id) {
    return apiGet(`transactions/${id}`, { node });
  },

  /**
   * @param {string} node
   * @param {string} id
   */
  getTransactionDetails(node, id) {
    return apiGet(`transactions/${id}/details`, { node });
  },

  /** @param {string} node */
  getTransactions(node) {
    return apiGet("transactions", { node });
  },
};
