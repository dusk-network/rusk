/**
 * @typedef {Object} TimeUnit
 * @property {Number} factor
 * @property {Intl.RelativeTimeFormatUnit} name
 */

/* eslint-disable sort-keys */

/** @type {TimeUnit[]} */
const units = [
  { name: "year", factor: 1000 * 60 * 60 * 24 * 365 },
  { name: "month", factor: 1000 * 60 * 60 * 24 * 30 },
  { name: "week", factor: 1000 * 60 * 60 * 24 * 7 },
  { name: "day", factor: 1000 * 60 * 60 * 24 },
  { name: "hour", factor: 1000 * 60 * 60 },
  { name: "minute", factor: 1000 * 60 },
  { name: "second", factor: 1000 },
];

/* eslint-enable sort-keys */

/**
 * @private
 * @param {Number} diff
 * @returns {TimeUnit}
 */
function getTimeUnit(diff) {
  for (const unit of units) {
    if (Math.abs(diff) >= unit.factor) {
      return unit;
    }
  }

  return units[6];
}

/**
 * @param {Date} date
 * @param {"long" | "short" | "narrow"} style
 * @return {String}
 */
function getRelativeTimeString(date, style) {
  const rtf = new Intl.RelativeTimeFormat("en", {
    localeMatcher: "best fit",
    numeric: "auto",
    style,
  });
  const diff = date.getTime() - Date.now();
  const unit = getTimeUnit(diff);

  return rtf.format(Math.round(diff / unit.factor), unit.name);
}

export default getRelativeTimeString;
