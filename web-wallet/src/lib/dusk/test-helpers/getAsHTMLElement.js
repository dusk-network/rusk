/**
 * Utility function to force the type checker to see
 * the return value of `querySelector` as a HTMLElement.
 *
 * @param {HTMLElement} container
 * @param {String} selector
 * @returns {HTMLElement}
 */
const getAsHTMLElement = (container, selector) =>
  /** @type {HTMLElement} */ (container.querySelector(selector));

export default getAsHTMLElement;
