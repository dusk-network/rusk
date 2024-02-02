import {
	afterEach,
	describe,
	expect,
	it
} from "vitest";
import {
	cleanup,
	render
} from "@testing-library/svelte";

import { Words } from "..";

describe("Words", () => {
	afterEach(cleanup);

	it("should render the \"Words\" component with underscores for empty string", () => {
		const { container } = render(Words, { props: { words: ["", "", ""] } });

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should render the \"Words\" component with the passed words", () => {
		const { container } = render(Words, { props: { words: ["snow", "winter", "christmas"] } });

		expect(container.firstChild).toMatchSnapshot();
	});
});
