import {
	afterEach,
	describe,
	expect,
	it,
	vi
} from "vitest";
import {
	cleanup,
	fireEvent,
	render
} from "@testing-library/svelte";
import { GasControls } from "..";

/**
 * @param {HTMLElement} input
 * @param {*} value
 * @returns {Promise<boolean>}
 */
const fireInput = (input, value) => fireEvent.input(input, { target: { value } });

/** @param {HTMLElement} element */
function asInput (element) {
	// eslint-disable-next-line no-extra-parens
	return /** @type {HTMLInputElement} */ (element);
}

describe("GasControls", () => {
	const baseProps = {
		limit: 20,
		limitLower: 10,
		limitUpper: 100,
		price: 10,
		priceLower: 1
	};
	const baseOptions = {
		props: baseProps,
		target: document.body
	};

	const eventHandler = vi.fn();

	afterEach(() => {
		cleanup();
		eventHandler.mockClear();
	});

	it("should render the `GasControls` component", () => {
		const { container, getByLabelText } = render(GasControls, baseOptions);
		const priceInput = asInput(getByLabelText(/price/i));
		const limitInput = asInput(getByLabelText(/limit/i));

		expect(priceInput.max).toBe(baseProps.limit.toString());
		expect(priceInput.min).toBe(baseProps.priceLower.toString());
		expect(limitInput.max).toBe(baseProps.limitUpper.toString());
		expect(limitInput.min).toBe(baseProps.limitLower.toString());
		expect(container).toMatchSnapshot();
	});

	it("should dispatch a \"gasSettings\" event when the price or the limit are changed with valid gas settings", async () => {
		const { component, getByLabelText } = render(GasControls, baseOptions);
		const priceInput = asInput(getByLabelText(/price/i));
		const limitInput = asInput(getByLabelText(/limit/i));

		component.$on("gasSettings", eventHandler);

		await fireInput(priceInput, 15);

		expect(eventHandler).toHaveBeenCalledTimes(1);
		expect(eventHandler.mock.lastCall[0].detail).toStrictEqual({
			isValidGas: true,
			limit: baseProps.limit,
			price: 15
		});
		expect(priceInput.valueAsNumber).toBe(15);

		await fireInput(limitInput, 25);

		expect(eventHandler).toHaveBeenCalledTimes(2);
		expect(eventHandler.mock.lastCall[0].detail).toStrictEqual({
			isValidGas: true,
			limit: 25,
			price: 15
		});
		expect(limitInput.valueAsNumber).toBe(25);
		expect(priceInput.max).toBe("25");
	});

	it("should dispatch a \"gasSettings\" event when the price or the limit are changed with invalid gas settings", async () => {
		const { component, getByLabelText } = render(GasControls, baseOptions);
		const priceInput = asInput(getByLabelText(/price/i));
		const limitInput = asInput(getByLabelText(/limit/i));

		component.$on("gasSettings", eventHandler);

		await fireInput(priceInput, 25);

		expect(eventHandler).toHaveBeenCalledTimes(1);
		expect(eventHandler.mock.lastCall[0].detail).toStrictEqual({
			isValidGas: false,
			limit: baseProps.limit,
			price: 25
		});
		expect(priceInput.valueAsNumber).toBe(25);

		await fireInput(limitInput, 105);

		expect(eventHandler).toHaveBeenCalledTimes(2);
		expect(eventHandler.mock.lastCall[0].detail).toStrictEqual({
			isValidGas: false,
			limit: 105,
			price: 25
		});
		expect(limitInput.valueAsNumber).toBe(105);
		expect(priceInput.max).toBe("105");
	});
});
