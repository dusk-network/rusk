import {
	afterEach,
	describe,
	expect,
	it
} from "vitest";
import { cleanup, render } from "@testing-library/svelte";

import { GasFee } from "..";
import { get } from "svelte/store";
import { settingsStore } from "$lib/stores";
import { createCurrencyFormatter } from "$lib/dusk/currency";

describe("GasFee", () => {
	const settings = get(settingsStore);
	const duskFormatter = createCurrencyFormatter(settings.language, "DUSK", 9);
	const fee = duskFormatter(settings.gasPrice * settings.gasLimit * 0.000000001);

	afterEach(cleanup);

	it("renders the GasFee component", () => {
		const baseProps = {
			fee: fee
		};
		const { container } = render(GasFee, baseProps);

		expect(container.querySelector(".gas-fee__amount-value")?.innerHTML).toBe(fee);

		expect(container.firstChild).toMatchSnapshot();
	});
});
