import {
	afterEach,
	describe,
	expect,
	it
} from "vitest";
import { cleanup, render } from "@testing-library/svelte";

import { Stepper } from "..";

describe("Stepper", () => {
	afterEach(cleanup);

	it("renders the Stepper component with two steps", () => {
		const { container } = render(Stepper, { props: { steps: 2, activeStep: 0 } });

		expect(container.firstChild).toMatchSnapshot();
	});

	it("renders the Stepper component with a completed step", () => {
		const { container } = render(Stepper, { props: { steps: 2, activeStep: 1 } });

		expect(container.firstChild).toMatchSnapshot();
	});

	it("renders the Stepper component with five steps", () => {
		const { container } = render(Stepper, { props: { steps: 5, activeStep: 0 } });

		expect(container.firstChild).toMatchSnapshot();
	});

	it("renders the Stepper component with five steps, with the third one being active, and the first two – completed", () => {
		const { container } = render(Stepper, { props: { steps: 5, activeStep: 3 } });

		expect(container.firstChild).toMatchSnapshot();
	});
});
