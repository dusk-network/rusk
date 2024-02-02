import { render } from "@testing-library/svelte";

import SlotContent from "./SlotContent.svelte";

/**
 * @callback CreateRenderer
 * @param {Parameters<render>[0]} Component
 * @param {Parameters<render>[1]} options
 * @param {Parameters<render>[2]} [renderOptions]
 * @returns {ReturnType<render>}
 */

/**
 * @typedef {Object} DefaultSlot
 * @property {String} default
 */

/**
 * @param {DefaultSlot} slots
 * @returns {CreateRenderer}
 */
const renderWithSlots = slots => (Component, options, renderOptions) => render(
	SlotContent,
	{ ...options, props: { Component, componentOptions: options?.props, text: slots.default } },
	renderOptions
);

export default renderWithSlots;
