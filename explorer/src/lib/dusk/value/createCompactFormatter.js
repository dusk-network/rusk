/**
 * Creates a locale aware currency formatter for fiat or DUSK
 *
 * @param {String} locale A BCP 47 language tag
 * @returns {(value: number | bigint) => string}
 */
const createFormatter = (locale) => {
  const formatter = new Intl.NumberFormat(locale, {
    compactDisplay: "short",
    maximumFractionDigits: 1,
    notation: "compact",
  });

  return (value) => formatter.format(value);
};

export default createFormatter;
