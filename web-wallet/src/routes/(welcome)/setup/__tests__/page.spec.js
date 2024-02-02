import {
	afterEach,
	describe,
	expect,
	it
} from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import Setup from "../+page.svelte";

describe("Setup", () => {
	afterEach(cleanup);

	it("should render the Setup page", () => {
		const { container } = render(Setup, {});

		expect(container.firstChild).toMatchSnapshot();
	});
});
