import {
	afterAll,
	afterEach,
	describe,
	expect,
	it,
	vi
} from "vitest";
import { cleanup, render } from "@testing-library/svelte";

import mockedWalletStore from "../../__mocks__/mockedWalletStore";
import { stakeInfo, transactions } from "$lib/mock-data";

import Dashboard from "../+page.svelte";
import { walletStore } from "$lib/stores";

vi.mock("$lib/stores", async importOriginal => {
	/** @type {import("$lib/stores/stores").WalletStore} */
	const original = await importOriginal();

	return {
		...original,
		walletStore: {
			...mockedWalletStore,
			getTransactionsHistory: () => Promise.resolve(transactions),
			getStakeInfo: () => Promise.resolve(stakeInfo)
		}
	};
});

vi.useFakeTimers();

describe("Dashboard", () => {
	afterEach(cleanup);

	afterAll(() => {
		vi.doUnmock("$lib/stores");
	});

	const currentPrice = { usd: 0.5 };

	it("should render the dashboard page and show a throbber while transactions are loading", () => {
		const { container } = render(Dashboard, { data: { currentPrice } });

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should render the dashboard page with the transactions after they are loaded", async () => {
		const { container } = render(Dashboard, { data: { currentPrice } });

		await vi.advanceTimersToNextTimerAsync();

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should render a card when there is an error getting transactions", async () => {
		const someError = new Error("some error message");
		const walletSpy = vi.spyOn(walletStore, "getTransactionsHistory").mockRejectedValue(someError);
		const { container } = render(Dashboard, { data: { currentPrice } });

		await vi.advanceTimersToNextTimerAsync();

		expect(container.firstChild).toMatchSnapshot();

		walletSpy.mockRestore();
	});
});
