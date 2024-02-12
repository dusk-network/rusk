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

	it("should dispatch a \"setGasSettings\" event when the price or the limit are changed", async () => {
		const { component, getByLabelText } = render(GasControls, baseOptions);
		const priceInput = asInput(getByLabelText(/price/i));
		const limitInput = asInput(getByLabelText(/limit/i));

		component.$on("setGasSettings", eventHandler);

		await fireInput(priceInput, 15);

		expect(eventHandler).toHaveBeenCalledTimes(1);
		expect(eventHandler.mock.lastCall[0].detail).toStrictEqual({
			limit: baseProps.limit,
			price: 15
		});
		expect(priceInput.valueAsNumber).toBe(15);

		await fireInput(limitInput, 25);

		expect(eventHandler).toHaveBeenCalledTimes(2);
		expect(eventHandler.mock.lastCall[0].detail).toStrictEqual({
			limit: 25,
			price: 15
		});
		expect(limitInput.valueAsNumber).toBe(25);
		expect(priceInput.max).toBe("25");
	});

	it("should dispatch a \"gasSettingsValidity\" event when the price or limit are changed", async () => {
		const { component, getByLabelText } = render(GasControls, baseOptions);
		const priceInput = asInput(getByLabelText(/price/i));
		const limitInput = asInput(getByLabelText(/limit/i));

		component.$on("gasSettingsValidity", eventHandler);

		await fireInput(priceInput, 15);

		expect(eventHandler).toHaveBeenCalledTimes(1);
		expect(eventHandler.mock.lastCall[0].detail).toBe(true);
		expect(priceInput.valueAsNumber).toBe(15);

		await fireInput(limitInput, 25);

		expect(eventHandler).toHaveBeenCalledTimes(3);
		expect(eventHandler.mock.lastCall[0].detail).toBe(true);
		expect(limitInput.valueAsNumber).toBe(25);
		expect(priceInput.max).toBe("25");

		await fireInput(priceInput, 30);
		expect(eventHandler).toHaveBeenCalledTimes(4);

		expect(eventHandler.mock.lastCall[0].detail).toBe(false);
		expect(priceInput.valueAsNumber).toBe(30);

		await fireInput(limitInput, 105);
		expect(eventHandler).toHaveBeenCalledTimes(6);
		expect(eventHandler.mock.lastCall[0].detail).toBe(false);
		expect(limitInput.valueAsNumber).toBe(105);
	});

	it("should convert the inputted price to integer, clamp it within its limits, dispatch the event and update the viewed value on blur", async () => {
		const { component, getByLabelText } = render(GasControls, baseOptions);
		const priceInput = asInput(getByLabelText(/price/i));
		const expectedPrice1 = baseProps.priceLower;
		const expectedPrice2 = baseProps.limit;

		component.$on("setGasSettings", eventHandler);

		await fireInput(priceInput, "foo");

		expect(eventHandler).toHaveBeenCalledTimes(1);
		expect(eventHandler.mock.lastCall[0].detail).toStrictEqual({
			limit: baseProps.limit,
			price: expectedPrice1
		});
		expect(priceInput.valueAsNumber).toBeNaN();

		await fireEvent.blur(priceInput);

		expect(priceInput.valueAsNumber).toBe(expectedPrice1);

		await fireInput(priceInput, 0);

		expect(eventHandler).toHaveBeenCalledTimes(2);
		expect(eventHandler.mock.lastCall[0].detail).toStrictEqual({
			limit: baseProps.limit,
			price: expectedPrice1
		});
		expect(priceInput.valueAsNumber).toBe(0);

		await fireEvent.blur(priceInput);

		expect(priceInput.valueAsNumber).toBe(expectedPrice1);

		await fireInput(priceInput, baseProps.limit * 2);

		expect(eventHandler).toHaveBeenCalledTimes(3);
		expect(eventHandler.mock.lastCall[0].detail).toStrictEqual({
			limit: baseProps.limit,
			price: expectedPrice2
		});
		expect(priceInput.valueAsNumber).toBe(baseProps.limit * 2);

		await fireEvent.blur(priceInput);

		expect(priceInput.valueAsNumber).toBe(expectedPrice2);
	});

	it("should convert the inputted limit to integer, clamp it within its limits, dispatch the event and update the viewed value on blur", async () => {
		const { component, getByLabelText } = render(GasControls, baseOptions);
		const priceInput = asInput(getByLabelText(/price/i));
		const limitInput = asInput(getByLabelText(/limit/i));
		const expectedLimit1 = baseProps.limitLower;
		const expectedLimit2 = baseProps.limitUpper;

		component.$on("setGasSettings", eventHandler);

		await fireInput(limitInput, "foo");

		expect(eventHandler).toHaveBeenCalledTimes(1);
		expect(eventHandler.mock.lastCall[0].detail).toStrictEqual({
			limit: expectedLimit1,
			price: baseProps.price
		});
		expect(limitInput.valueAsNumber).toBeNaN();
		expect(priceInput.max).toBe(expectedLimit1.toString());

		await fireEvent.blur(limitInput);

		expect(limitInput.valueAsNumber).toBe(expectedLimit1);

		await fireInput(limitInput, 0);

		expect(eventHandler).toHaveBeenCalledTimes(2);
		expect(eventHandler.mock.lastCall[0].detail).toStrictEqual({
			limit: expectedLimit1,
			price: baseProps.price
		});
		expect(limitInput.valueAsNumber).toBe(0);
		expect(priceInput.max).toBe(expectedLimit1.toString());

		await fireEvent.blur(limitInput);

		expect(limitInput.valueAsNumber).toBe(expectedLimit1);

		await fireInput(limitInput, baseProps.limitUpper * 2);

		expect(eventHandler).toHaveBeenCalledTimes(3);
		expect(eventHandler.mock.lastCall[0].detail).toStrictEqual({
			limit: expectedLimit2,
			price: baseProps.price
		});
		expect(limitInput.valueAsNumber).toBe(baseProps.limitUpper * 2);
		expect(priceInput.max).toBe(expectedLimit2.toString());

		await fireEvent.blur(limitInput);

		expect(limitInput.valueAsNumber).toBe(expectedLimit2);
	});

	it("should update the price value if a limit change makes it bigger than the limit", async () => {
		const props = {
			...baseProps,
			limitLower: baseProps.price / 2
		};
		const { component, getByLabelText } = render(GasControls, { ...baseOptions, props });
		const priceInput = asInput(getByLabelText(/price/i));
		const limitInput = asInput(getByLabelText(/limit/i));

		component.$on("setGasSettings", eventHandler);

		await fireInput(limitInput, props.limitLower);

		expect(eventHandler).toHaveBeenCalledTimes(1);
		expect(eventHandler.mock.lastCall[0].detail).toStrictEqual({
			limit: props.limitLower,
			price: props.limitLower
		});
		expect(limitInput.valueAsNumber).toBe(props.limitLower);
		expect(priceInput.valueAsNumber).toBe(props.limitLower);
		expect(priceInput.max).toBe(props.limitLower.toString());

		await fireEvent.blur(limitInput);

		expect(limitInput.valueAsNumber).toBe(props.limitLower);
	});
});
