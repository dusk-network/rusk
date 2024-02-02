/**
 * Checks if two arrays are equal.
 * This function performs a SameValue equality comparison
 * on each pair of corresponding elements in the two arrays.
 *
 * @param {any[]} arr1 - The first array to be compared.
 * @param {any[]} arr2 - The second array to be compared.
 * @returns {boolean} True if the arrays are of the same length
 * and all corresponding elements are equal, false otherwise.
 */

function arraysEqual (arr1, arr2) {
	if (arr1.length !== arr2.length) {
		return false;
	}

	for (let i = 0; i < arr1.length; i++) {
		if (!Object.is(arr1[i], arr2[i])) {
			return false;
		}
	}

	return true;
}

export default arraysEqual;
