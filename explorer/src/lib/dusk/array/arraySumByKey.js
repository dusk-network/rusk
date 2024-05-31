import { compose, pluck } from "lamb";

import { arraySum } from ".";

/**
 * Builds a function that sums the values in the given key in an array of objects.
 * The built function will throw a `TypeError` if the received array is empty.
 *
 * @example
 * const scores = [
 *     { score: 7, user: "John" },
 *     { score: 9, user: "Jane" },
 *     { score: 5, user: "Mario" }
 * ];
 * const sumScores = arraySumByKey("score");
 *
 * sumScores(scores) // => 21
 *
 * @template {string} K
 * @param {K} key
 * @returns {(source: ArrayLike<{ [P in K]: number } & Record<PropertyKey, any>>) => number}
 */
const arraySumByKey = (key) => compose(arraySum, pluck(key));

export default arraySumByKey;
