import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { get } from "svelte/store";

import mockedWalletStore from "../../__mocks__/mockedWalletStore";
import { stakeInfo, transactions } from "$lib/mock-data";
import { createCurrencyFormatter } from "$lib/dusk/currency";

import Dashboard from "../+page.svelte";
import { walletStore } from "$lib/stores";

global.ResizeObserver = vi.fn().mockImplementation(() => ({
  disconnect: vi.fn(),
  observe: vi.fn(),
  unobserve: vi.fn(),
}));

vi.mock("$lib/stores", async (importOriginal) => {
  /** @type {import("$lib/stores/stores").WalletStore} */
  const original = await importOriginal();

  return {
    ...original,
    walletStore: {
      ...mockedWalletStore,
      getStakeInfo: () => Promise.resolve(stakeInfo),
      getTransactionsHistory: () => Promise.resolve(transactions),
    },
  };
});

vi.useFakeTimers();

describe("Dashboard", () => {
  afterEach(cleanup);

  afterAll(() => {
    vi.doUnmock("$lib/stores");
  });

  const usdPrice = 0.5;
  const expectedFiat = get(walletStore).balance.value * usdPrice;
  const formatter = createCurrencyFormatter("en", "usd", 2);
  const baseProps = {
    data: { currentPrice: Promise.resolve({ usd: usdPrice }) },
  };

  it("should render the dashboard page and show a throbber while transactions are loading", async () => {
    const { container } = render(Dashboard, baseProps);

    expect(container.querySelector(".dusk-balance__fiat")).toBeNull();
    expect(container.firstChild).toMatchSnapshot();

    await vi.advanceTimersToNextTimerAsync();

    expect(container.querySelector(".dusk-balance__fiat")).toHaveTextContent(
      formatter(expectedFiat)
    );
  });

  it("should render the dashboard page with the transactions after they are loaded", async () => {
    const { container } = render(Dashboard, baseProps);

    await vi.advanceTimersToNextTimerAsync();

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should render a card when there is an error getting transactions", async () => {
    const someError = new Error("some error message");
    const walletSpy = vi
      .spyOn(walletStore, "getTransactionsHistory")
      .mockRejectedValue(someError);
    const { container } = render(Dashboard, baseProps);

    await vi.advanceTimersToNextTimerAsync();

    expect(container.firstChild).toMatchSnapshot();

    walletSpy.mockRestore();
  });
});
