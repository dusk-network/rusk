/**
 * Returns the decimal separator for the current locale.  Typically a comma or a period.
 *
 * @returns {string}
 */
const getDecimalSeparator = () => {
  return (0.1).toLocaleString().slice(1, 2);
};

export default getDecimalSeparator;
