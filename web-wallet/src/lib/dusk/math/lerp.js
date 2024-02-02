/**
 * Performs a [linear interpolation]{@link https://en.wikipedia.org/wiki/Linear_interpolation}
 * between the given `a` and `b` numbers using the given normal value `n`.
 *
 * A value of `0` for `n` will return `a`.
 * A value of `1` for `n` will return `b`.
 *
 * The resulting interpolation in a motion will be "smoother" for
 * values near `0` and "sharper" for values near `1`.
 *
 * @param {Number} a The starting value
 * @param {Number} b The destination value
 * @param {Number} n The normal value (between `0` and `1`)
 * @returns {Number}
 */
const lerp = (a, b, n) => (1 - n) * a + n * b;

export default lerp;
