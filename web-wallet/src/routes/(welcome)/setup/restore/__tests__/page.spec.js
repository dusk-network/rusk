import {
	afterEach,
	describe,
	expect,
	it
} from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import Restore from "../+page.svelte";

describe("Restore", () => {
	afterEach(cleanup);

	it("should render the Terms of Service step of the Restore flow", () => {
		const { container } = render(Restore);

		expect(container.firstChild).toMatchSnapshot();
	});
});
