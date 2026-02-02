import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup } from "@testing-library/svelte";
import { renderWithSimpleContent } from "$lib/dusk/test-helpers";
import mockedWalletStore from "$lib/mocks/mockedWalletStore";
import { stakeInfo, transactions } from "$lib/mock-data";

import Dashboard from "../+page.svelte";
import { walletStore } from "$lib/stores";

vi.mock("$lib/stores", async (importOriginal) => {
  /** @type {WalletStore} */
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

  it("should render the dashboard page with the transactions after they are loaded", async () => {
    const { container } = renderWithSimpleContent(Dashboard, {});

    await vi.advanceTimersToNextTimerAsync();

    expect(container).toMatchSnapshot();
  });

  it("should render a card when there is an error getting transactions", async () => {
    const someError = new Error("some error message");
    const walletSpy = vi
      .spyOn(walletStore, "getTransactionsHistory")
      .mockRejectedValue(someError);
    const { container } = renderWithSimpleContent(Dashboard, {});

    await vi.advanceTimersToNextTimerAsync();

    expect(container).toMatchSnapshot();

    walletSpy.mockRestore();
  });
});
