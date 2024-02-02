import {
	afterEach,
	describe,
	expect,
	it
} from "vitest";
import { cleanup, render } from "@testing-library/svelte";

import { Badge } from "..";

describe("Badge", () => {
	const baseProps = {
		text: "Badge"
	};

	afterEach(cleanup);

	it("should render the Badge component using the type \"neutral\" as a default", () => {
		const { container } = render(Badge, baseProps);

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should render the Badge component using the type \"warning\" variant", () => {
		const { container } = render(Badge, { ...baseProps, variant: "warning" });

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should render the Badge component using the type \"error\" variant", () => {
		const { container } = render(Badge, { ...baseProps, variant: "error" });

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should render the Badge component using the type \"success\" variant", () => {
		const { container } = render(Badge, { ...baseProps, variant: "success" });

		expect(container.firstChild).toMatchSnapshot();
	});
});
