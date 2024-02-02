/**
 * Calculates the number of characters to display based on the screen width,
 * with adjustable parameters for minimum and maximum screen widths and character counts.
 * It uses linear interpolation to determine the number of characters relative to the screen width.
 *
 * @param {Number} width - The current width of the screen in pixels.
 * @param {Number} [minWidth=320] - The minimum screen width to consider, defaulting to 320 pixels.
 * @param {Number} [maxWidth=720] - The maximum screen width to consider, defaulting to 720 pixels.
 * @param {Number} [minCharacters=5] - The minimum number of characters to display,
 * 									   defaulting to 5 characters.
 * @param {Number} [maxCharacters=18] - The maximum number of characters to display,
 * 										defaulting to 18 characters.
 * @returns {Number} The calculated number of characters to be displayed,
 * 					 rounded to the nearest integer.
 * 					 This value is clamped between the minimum and maximum number of characters.
 *
 * @example
 * // if screen width is 320px
 * calculateAdaptiveCharCount(320); // returns 5
 *
 * @example
 * // if screen width is 520px
 * calculateAdaptiveCharCount(520); // returns a value between 5 and 20
 *
 * @example
 * // if screen width is 720px or more
 * calculateAdaptiveCharCount(720); // returns 20
 *
 * @example
 * // using custom parameters
 * calculateAdaptiveCharCount(500, 400, 800, 10, 30); //
 * returns a calculated value based on custom parameters
 */

function calculateAdaptiveCharCount (
	width,
	minWidth = 320,
	maxWidth = 640,
	minCharacters = 5,
	maxCharacters = 18
) {
	const characters =
		minCharacters + (width - minWidth) * (maxCharacters - minCharacters) / (maxWidth - minWidth);

	return Math.round(Math.max(minCharacters, Math.min(characters, maxCharacters)));
}

export default calculateAdaptiveCharCount;
