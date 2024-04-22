/**
 * Creates a locale aware currency formatter for fiat or DUSK
 *
 * @param {String} locale A BCP 47 language tag
 * @param {String} currency An ISO 4217 currency or "DUSK"
 * @param {Number} digits The minimum fraction digits that should display
 * @returns {(value: number | bigint) => string}
 */
const createFormatter = (locale, currency, digits) => {
  const formatter =
    currency.toUpperCase() === "DUSK"
      ? new Intl.NumberFormat(locale, { minimumFractionDigits: digits })
      : new Intl.NumberFormat(locale, {
          currency: currency,
          maximumFractionDigits: 9,
          minimumFractionDigits: 2,
          style: "currency",
        });

  return (value) => formatter.format(value);
};

export default createFormatter;
