import {
	afterEach,
	describe,
	expect,
	it
} from "vitest";
import { cleanup, render } from "@testing-library/svelte";

import { ProgressBar } from "..";

describe("ProgressBar", () => {
	afterEach(cleanup);

	it("renders the ProgressBar component with no current percentage set", () => {
		const { container } = render(ProgressBar);

		expect(container.firstChild).toMatchSnapshot();
	});

	it("renders the Stepper component with current percentage set as zero", () => {
		const { container } = render(ProgressBar, { props: { currentPercentage: 0 } });

		expect(container.firstChild).toMatchSnapshot();
	});

	it("re-renders the Stepper component when the current percentage property changes", async () => {
		const { component, container } = render(ProgressBar, { props: { currentPercentage: 0 } });

		expect(container.firstChild).toMatchSnapshot();

		await component.$set({ currentPercentage: 50 });

		expect(container.firstChild).toMatchSnapshot();
	});
});
