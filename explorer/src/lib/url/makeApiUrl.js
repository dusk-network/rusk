import { ensureTrailingSlash } from "$lib/dusk/string";

/**
 * @param {string} endpoint
 * @param {Record<string, any> | undefined} params
 * @returns {URL}
 */
const makeApiUrl = (endpoint, params) =>
  new URL(
    `${endpoint}?${new URLSearchParams(params)}`,
    ensureTrailingSlash(import.meta.env.VITE_API_ENDPOINT)
  );

export default makeApiUrl;
