/**
 * Cleans the input string
 * Returns a valid number in string form with the correct decimal separator according to the locale
 *
 * @param {string} amount
 * @param {string} separator
 * @returns {string}
 */
const cleanNumberString = (amount, separator) => {
  console.log("Cleaning number string:", { amount, separator });
  console.log(typeof amount, typeof separator);
  const regex = new RegExp(`[^\\d${separator}]+`, "g"); // Remove any character that are not digits or the decimal separator
  const regex2 = new RegExp(`(?<=\\${separator}.*)\\${separator}`, "g"); // Remove all but the first decimal separator
  const regex3 = new RegExp(/^0+(?=\d)/); // Remove leading zeros

  return amount.replace(regex, "").replace(regex2, "").replace(regex3, "");
};

export default cleanNumberString;
