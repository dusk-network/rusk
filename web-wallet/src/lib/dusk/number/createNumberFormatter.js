/**
 * Creates a locale aware number formatter
 *
 * @param {String} locale A BCP 47 language tag
 * @param {Number|Undefined} digits The maximum fraction digits that should display
 * @returns {(value: number | bigint) => string}
 */
const createNumberFormatter = (locale, digits = undefined) => {
  const formatter = new Intl.NumberFormat(locale, {
    maximumFractionDigits: digits,
    style: "decimal",
  });

  return (value) => formatter.format(value);
};

export default createNumberFormatter;
