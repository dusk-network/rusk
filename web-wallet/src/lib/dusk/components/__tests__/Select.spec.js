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
	render,
	screen
} from "@testing-library/svelte";

import { Select } from "..";

const stringOptions = ["one", "two", "three", "four"];

/** @type {SelectOption[]} */
const objectOptions = [
	{ label: "one", value: "1" },
	{ label: "two", value: "2" },
	{ disabled: true, label: "three", value: "3" },
	{ label: "four", value: "4" }
];

/** @type {GroupedSelectOptions} */
const groupedOptions = {
	"Group one": [
		{ label: "one", value: "1" },
		{ label: "two", value: "2" },
		{ disabled: true, label: "three", value: "3" }
	],
	"Group two": [
		{ label: "four", value: "4" },
		{ label: "five", value: "5" }
	]
};

/** @type {GroupedSelectOptions} */
const groupedOptionsStrings = {
	"Group one": ["one", "two", "three", "four"],
	"Group two": ["five", "six", "seven"]
};

describe("Select", () => {
	const baseProps = {
		options: stringOptions
	};
	const baseOptions = {
		props: baseProps,
		target: document.body
	};

	afterEach(cleanup);

	it("should render the Select component", () => {
		const { container } = render(Select, baseOptions);

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should accept a change event handler", async () => {
		const changeHandler = vi.fn();
		const props = {
			...baseProps,
			"data-testid": "my-select",
			"value": "two"
		};
		const { component } = render(Select, { ...baseOptions, props });

		/** @type {HTMLSelectElement} */
		const select = screen.getByTestId("my-select");
		const target = select.querySelector("option[value='four']");

		component.$on("change", changeHandler);

		await fireEvent.change(select, { target });

		expect(changeHandler).toHaveBeenCalledTimes(1);
		expect(changeHandler).toHaveBeenCalledWith(expect.any(Event));
		expect(select.value).toBe("four");
	});

	it("should pass additional class names and attributes to the rendered element", () => {
		const props = {
			...baseProps,
			className: "foo bar",
			id: "some-id"
		};
		const { container } = render(Select, { ...baseOptions, props });

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should accept an array of objects as options", () => {
		const props = {
			...baseProps,
			options: objectOptions
		};
		const { container } = render(Select, { ...baseOptions, props });

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should use the value as label if an object is missing it", () => {
		const objectOptions2 = objectOptions.concat({ value: "5" });
		const props = {
			...baseProps,
			options: objectOptions2
		};
		const { container } = render(Select, { ...baseOptions, props });

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should accept an empty string as label and use it instead of falling back to the value", () => {
		const objectOptions2 = objectOptions.concat({ label: "", value: "5" });
		const props = {
			...baseProps,
			options: objectOptions2
		};
		const { container } = render(Select, { ...baseOptions, props });

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should accept an array of strings as `options` and use each string as both label and value", () => {
		const props = {
			...baseProps,
			options: groupedOptionsStrings["Group one"]
		};
		const { container } = render(Select, { ...baseOptions, props });

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should accept a grouped object as `options` and create option groups", () => {
		const props = {
			...baseProps,
			options: groupedOptions
		};
		const { container } = render(Select, { ...baseOptions, props });

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should accept an array of string as values of a grouped object", () => {
		const props = {
			...baseProps,
			options: groupedOptionsStrings
		};
		const { container } = render(Select, { ...baseOptions, props });

		expect(container.firstChild).toMatchSnapshot();
	});
});
