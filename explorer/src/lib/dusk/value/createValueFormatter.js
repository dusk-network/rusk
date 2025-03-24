/**
 * Creates a locale aware currency formatter for fiat or DUSK
 *
 * @param {String} locale A BCP 47 language tag
 * @param {Number} minFractionDigits The minimum fraction digits that should display
 * @param {Number} maxFractionDigits The maximum fraction digits that should display
 * @returns {(value: number | bigint) => string}
 */
const createFormatter = (
  locale,
  minFractionDigits = 0,
  maxFractionDigits = 9
) => {
  const formatter = new Intl.NumberFormat(locale, {
    maximumFractionDigits: maxFractionDigits,
    minimumFractionDigits: minFractionDigits,
  });

  return (value) => formatter.format(value);
};

export default createFormatter;
