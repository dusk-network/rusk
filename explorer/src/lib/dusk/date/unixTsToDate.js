/**
 * @param {number} ts
 * @returns {Date}
 */
const unixTsToDate = (ts) => new Date(ts * 1000);

export default unixTsToDate;
