/**
 * Creates a locale aware currency formatter for fiat or DUSK
 *
 * @param {String} locale A BCP 47 language tag
 * @returns {(value: number | bigint) => string}
 */
const createFormatter = (locale) => {
  const formatter = new Intl.NumberFormat(locale, {
    maximumFractionDigits: 9,
    minimumFractionDigits: 0,
  });

  return (value) => formatter.format(value);
};

export default createFormatter;
