import { reduceWith, sum } from "lamb";

/**
 * Sums the values in the given array.
 *
 * @example
 * arraySum([1, 2, 3, 4, 5]) // => 15
 *
 * @throws {TypeError} If the received array is empty.
 * @type {(source: number[]) => number}
 */
const arraySum = reduceWith(sum);

export default arraySum;
