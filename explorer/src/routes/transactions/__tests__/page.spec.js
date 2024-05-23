import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { get } from "svelte/store";

import { duskAPI } from "$lib/services";
import { transformTransaction } from "$lib/chain-info";
import { appStore } from "$lib/stores";
import { apiTransactions } from "$lib/mock-data";

import Transactions from "../+page.svelte";

describe("Transactions page", () => {
  vi.useFakeTimers();

  const { fetchInterval, network } = get(appStore);
  const getTransactionSpy = vi
    .spyOn(duskAPI, "getTransactions")
    .mockResolvedValue(apiTransactions.data.map(transformTransaction));

  afterEach(() => {
    cleanup();
    getTransactionSpy.mockClear();
  });

  afterAll(() => {
    vi.useRealTimers();
    getTransactionSpy.mockRestore();
  });

  it("should render the Transactions page, start polling for blocks and stop the polling when unmounted", async () => {
    const { container, unmount } = render(Transactions);

    expect(container.firstChild).toMatchSnapshot();
    expect(getTransactionSpy).toHaveBeenCalledTimes(1);
    expect(getTransactionSpy).toHaveBeenNthCalledWith(1, network);

    await vi.advanceTimersByTimeAsync(1);

    // snapshot with received data from APIs
    expect(container.firstChild).toMatchSnapshot();

    await vi.advanceTimersByTimeAsync(fetchInterval - 1);

    expect(getTransactionSpy).toHaveBeenCalledTimes(2);
    expect(getTransactionSpy).toHaveBeenNthCalledWith(2, network);

    await vi.advanceTimersByTimeAsync(fetchInterval);

    expect(getTransactionSpy).toHaveBeenCalledTimes(3);
    expect(getTransactionSpy).toHaveBeenNthCalledWith(3, network);

    unmount();

    await vi.advanceTimersByTimeAsync(fetchInterval * 10);

    expect(getTransactionSpy).toHaveBeenCalledTimes(3);
  });
});
