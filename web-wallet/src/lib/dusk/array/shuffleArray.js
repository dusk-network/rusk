import { randomInt } from "lamb";

/**
 * Shuffles an array using the Fisher-Yates algorithm.
 * @template T
 * @param {T[]} array - The array to be shuffled.
 * @returns {T[]} The shuffled array.
 */

function shuffleArray (array) {
	const shuffledArray = [...array];

	for (let i = shuffledArray.length - 1; i > 0; i--) {
		const j = randomInt(0, i);

		[shuffledArray[i], shuffledArray[j]] = [shuffledArray[j], shuffledArray[i]];
	}

	return shuffledArray;
}

export default shuffleArray;
