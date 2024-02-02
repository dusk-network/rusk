/**
 * Utility function to force the type checker to see
 * the return value of `querySelector` as a HTMLElement.
 *
 * @param {HTMLElement} container
 * @param {String} selector
 * @returns {HTMLElement}
 */
function getAsHTMLElement (container, selector) {
	// eslint-disable-next-line no-extra-parens
	return /** @type {HTMLElement} */ (container.querySelector(selector));
}

export default getAsHTMLElement;
