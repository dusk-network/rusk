import {
	afterEach,
	describe,
	expect,
	it
} from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { Card } from "..";

describe("Card", () => {
	afterEach(cleanup);

	it("renders the Card component with a heading", () => {
		const { container } = render(Card, { props: { heading: "My Card" } });

		expect(container.firstChild).toMatchSnapshot();
	});

	it("renders the the Card component with an icon when iconPath is provided", () => {
		const { container } = render(
			Card,
			{ props: { heading: "My Card", iconPath: "M3,3H21V21H3V3M5,5V19H19V5H5Z" } }
		);

		expect(container.firstChild).toMatchSnapshot();
	});
});
