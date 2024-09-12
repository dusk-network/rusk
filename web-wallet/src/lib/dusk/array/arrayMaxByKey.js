import { reduce } from "lamb";

/**
 * Gets the max value of the given key in an array of objects.
 *
 *
 * @example
 * const scores = [
 *     { score: 7, user: "John" },
 *     { score: 9, user: "Jane" },
 *     { score: 5, user: "Mario" }
 * ];
 * const getMaxScore = arrayMaxByKey("score");
 *
 * getMaxScore(scores) // => 9
 *
 * @template {string} K
 * @template {Record<PropertyKey, any> & Record<K, import("lamb").Ord>} S
 * @param {K} key
 * @returns {<const L extends S[]>(array: L) => L extends [] ? undefined : typeof array[number][K]}
 */
const arrayMaxByKey = (key) => (array) =>
  array.length === 0
    ? undefined
    : reduce(array, (r, c) => (r[key] > c[key] ? r : c))[key];

export default arrayMaxByKey;
