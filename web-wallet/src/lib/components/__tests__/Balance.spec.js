import {
	afterEach,
	describe,
	expect,
	it
} from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { skipIn } from "lamb";

import { Balance } from "..";

describe("Balance", () => {
	const baseProps = {
		fiatCurrency: "USD",
		fiatPrice: 10,
		locale: "en",
		tokenCurrency: "DUSK",
		tokens: 2000000
	};
	const baseOptions = {
		props: baseProps,
		target: document.body
	};

	afterEach(cleanup);

	it("renders the Balance component", () => {
		const { container } = render(Balance, baseOptions);

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should update the Balance component when the props change", () => {
		const { container, rerender } = render(Balance, baseOptions);

		expect(container.firstChild).toMatchSnapshot();

		rerender({
			fiatCurrency: "EUR",
			fiatPrice: 20,
			locale: "it",
			tokenCurrency: "DUSK",
			tokens: 4000000
		});

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should pass additional class names and attributes to the rendered element", () => {
		const props = {
			...baseProps,
			className: "foo bar",
			id: "balance"
		};
		const { container, rerender } = render(Balance, { ...baseOptions, props });

		expect(container.firstChild).toHaveClass("foo bar");
		expect(container.firstChild).toHaveAttribute("id", "balance");

		rerender({
			...props,
			className: "qux",
			id: "new-balance"
		});

		expect(container.firstChild).toHaveClass("qux");
		expect(container.firstChild).toHaveAttribute("id", "new-balance");
	});

	it("should skip rendering the fiat value if the fiat price is `undefined`", () => {
		const props = skipIn(baseProps, ["fiatPrice"]);
		const { container } = render(Balance, { ...baseOptions, props });

		expect(container.firstChild).toMatchSnapshot();
	});
});
