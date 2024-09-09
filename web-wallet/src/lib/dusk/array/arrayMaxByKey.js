import { compose, pluck } from "lamb";

import arrayMax from "./arrayMax";

/**
 * Gets the max numeric value of the given key in an array of objects.
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
 * @template {Record<PropertyKey, any> & Record<K, number>} S
 * @param {K} key
 * @returns {(list: S[]) => number}
 */
const arrayMaxByKey = (key) => compose(arrayMax, pluck(key));

export default arrayMaxByKey;
