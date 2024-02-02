import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import Transactions from "../+page.svelte";

describe("Dashboard", () => {
	afterEach(cleanup);

	const currentPrice = { usd: 0.5 };

	it("should render the transactions page", () => {
		const { container } = render(Transactions, { data: { currentPrice } });

		expect(container.firstChild).toMatchSnapshot();
	});
});
