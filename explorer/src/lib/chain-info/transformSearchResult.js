import { compose, getPathIn, mapWith } from "lamb";

/**
 * @template {Record<PropertyKey, any>} T
 * @template {string} U
 * @param {T} source
 * @param {U} path
 * @returns {Exclude<import("lamb").GetPath<T, U>, undefined> | []}
 */
const getPathOrEmptyArray = (source, path) => getPathIn(source, path) ?? [];

/** @type {(v: APISearchResult) => SearchResult[]} */
const transformSearchResult = compose(
  mapWith((entry) =>
    entry?.header?.hash
      ? { id: entry.header.hash, type: "block" }
      : { id: entry.txid, type: "transaction" }
  ),
  (v) => [
    ...getPathOrEmptyArray(v, "data.data.blocks"),
    ...getPathOrEmptyArray(v, "data.data.transactions"),
  ]
);

export default transformSearchResult;
