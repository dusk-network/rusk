import {
	afterEach,
	describe,
	expect,
	it
} from "vitest";
import { cleanup, render } from "@testing-library/svelte";

import { ContractStatusesList } from "..";

describe("ContractStatusesList", () => {
	const baseProps = {
		items: [{
			"label": "Spendable",
			"value": "99,899.999724165"
		}, {
			"label": "Total Locked",
			"value": "1,000.000000000"
		}, {
			"label": "Rewards",
			"value": "99,288.000000000"
		}]
	};
	const baseOptions = {
		props: baseProps,
		target: document.body
	};

	afterEach(cleanup);

	it("should render the `ContractStatusesList` component", () => {
		const { container } = render(ContractStatusesList, baseOptions);

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should be able to render the component without items", () => {
		const props = {
			...baseProps,
			items: []
		};
		const { container } = render(ContractStatusesList, { ...baseOptions, props });

		expect(container.firstChild).toMatchSnapshot();
	});
});
