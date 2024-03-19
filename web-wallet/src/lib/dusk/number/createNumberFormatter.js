/**
 * Creates a locale aware currency formatter for fiat or DUSK
 *
 * @param {String} locale A BCP 47 language tag
 * @param {Number|Undefined} digits The minimum fraction digits that should display
 * @returns {(value: number | bigint) => string}
 */
const createNumberFormatter = (locale, digits = undefined) => {
  const formatter = new Intl.NumberFormat(locale, {
    style: "decimal",
    maximumFractionDigits: digits,
  });

  return (value) => formatter.format(value);
};

export default createNumberFormatter;
