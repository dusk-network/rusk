import {
	afterAll,
	afterEach,
	describe,
	expect,
	it,
	vi
} from "vitest";
import { cleanup, render } from "@testing-library/svelte";

import LogoutPage from "../+page.svelte";

describe("Forced logout page", () => {
	const jsDomAlert = window.alert;

	window.alert = vi.fn();

	afterEach(cleanup);

	afterAll(() => {
		window.alert = jsDomAlert;
	});

	it("should render the page alert the user about the forced logout on mount", () => {
		const { container } = render(LogoutPage);

		expect(window.alert).toHaveBeenCalledTimes(1);
		expect(window.alert).toHaveBeenCalledWith(expect.any(String));
		expect(container.firstChild).toMatchSnapshot();
	});
});
