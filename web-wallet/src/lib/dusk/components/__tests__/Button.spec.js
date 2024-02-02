import {
	afterEach,
	describe,
	expect,
	it
} from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { mdiFolderOutline } from "@mdi/js";

import { Button } from "..";

describe("Button", () => {
	const baseProps = {
		text: "some text"
	};
	const baseOptions = {
		props: baseProps,
		target: document.body
	};

	afterEach(cleanup);

	it("should render the Button component using the type \"button\" as a default", () => {
		const { container } = render(Button, baseOptions);

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should render a button of the desired type", () => {
		["button", "reset", "submit", "toggle"].forEach(type => {
			const props = { ...baseProps, type };
			const { container } = render(Button, { ...baseOptions, props });

			expect(container.firstChild).toMatchSnapshot();
		});
	});

	it("should accept an active property for the toggle button type", () => {
		const props = { ...baseProps, active: true, type: "toggle" };
		const { container } = render(Button, { ...baseOptions, props });

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should pass additional class names and attributes to the rendered element", () => {
		const props = {
			...baseProps,
			className: "foo bar",
			id: "some-id"
		};
		const { container } = render(Button, { ...baseOptions, props });

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should render a button without a text", () => {
		const props = {
			...baseProps,
			text: ""
		};
		const { container } = render(Button, { ...baseOptions, props });

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should be able to render a button with an icon and text", () => {
		["after", "before"].forEach(position => {
			const props = {
				...baseProps,
				icon: {
					path: mdiFolderOutline,
					position
				}
			};
			const { container } = render(Button, { ...baseOptions, props });

			expect(container.firstChild).toMatchSnapshot();
		});
	});

	it("should be able to render a button with an icon only", () => {
		["after", "before"].forEach(position => {
			const props = {
				...baseProps,
				icon: {
					path: mdiFolderOutline,
					position
				},
				text: ""
			};
			const { container } = render(Button, { ...baseOptions, props });

			expect(container.firstChild).toMatchSnapshot();
		});
	});
});
