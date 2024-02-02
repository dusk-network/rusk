
/**
 * Finds the first N matches in an array of words that start with a given prefix.
 * @note The matching is case-sensitive.
 * @param {string[]} words – The array of words to search.
 * @param {string} prefix – The prefix to match.
 * @param {number} numMatches – The number of matches to find.
 * @returns {string[]} The first N matches.
 */
function findFirstNMatches (words, prefix, numMatches) {
	/**
	 * @type {string[]}
	 */
	const matches = [];

	if (numMatches <= 0) {
		return matches;
	}

	for (const word of words) {
		if (word.startsWith(prefix)) {
			matches.push(word);
		}

		if (matches.length === numMatches) {
			break;
		}
	}

	return matches;
}

export default findFirstNMatches;
