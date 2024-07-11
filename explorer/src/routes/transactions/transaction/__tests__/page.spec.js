import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";

import { resolveAfter } from "$lib/dusk/promise";
import { duskAPI } from "$lib/services";

import TransactionDetails from "../+page.svelte";

const marketDataSettleTime = vi.hoisted(() => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2024, 4, 20));

  return 100;
});
vi.mock("$lib/services", async (importOriginal) => {
  /** @type {import("$lib/services")} */
  const original = await importOriginal();
  const { transformTransaction } = await import("$lib/chain-info");
  const { apiMarketData, gqlTransaction, gqlTransactionDetails } = await import(
    "$lib/mock-data"
  );
  const { current_price: currentPrice, market_cap: marketCap } =
    apiMarketData.market_data;

  return {
    ...original,
    duskAPI: {
      ...original.duskAPI,
      getMarketData: () =>
        resolveAfter(marketDataSettleTime, { currentPrice, marketCap }),
      getTransaction: vi
        .fn()
        .mockResolvedValue(transformTransaction(gqlTransaction.tx)),
      getTransactionDetails: vi
        .fn()
        .mockResolvedValue(gqlTransactionDetails.tx.raw),
    },
  };
});

describe("Transaction Details", () => {
  afterEach(() => {
    cleanup();
    vi.clearAllMocks();
  });

  afterAll(() => {
    vi.useRealTimers();
    vi.doUnmock("$lib/services");
  });

  it("should render the Transaction details page and query the necessary info", async () => {
    const { container } = render(TransactionDetails);

    expect(container.firstChild).toMatchSnapshot();

    expect(duskAPI.getTransaction).toHaveBeenCalledTimes(1);
    expect(duskAPI.getTransactionDetails).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(marketDataSettleTime);

    // snapshot with received data from APIs
    expect(container.firstChild).toMatchSnapshot();
  });
});
