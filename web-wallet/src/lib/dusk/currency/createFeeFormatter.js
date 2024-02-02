/**
 * Creates a locale aware currency formatter for fees
 *
 * @param {String} locale A BCP 47 language tag
 * @returns {(value: number | bigint) => string}
 */
const createFormatter = (locale) => {
	const formatter = new Intl.NumberFormat(locale, {
		maximumFractionDigits: 9,
		minimumFractionDigits: 2
	});

	return value => formatter.format(value);
};

export default createFormatter;
