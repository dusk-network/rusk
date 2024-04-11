/**
 * Used to shorten a string by replacing the middle with an ellipsis.
 * @param {String} originalString
 * @param {Number} charactersToDisplay
 * @returns {String}
 */
function middleEllipsis(originalString, charactersToDisplay) {
  if (originalString.length <= 2 * charactersToDisplay) {
    return originalString;
  }

  return `${originalString.substring(
    0,
    charactersToDisplay
  )}...${originalString.substring(originalString.length - charactersToDisplay)}`;
}

export default middleEllipsis;
