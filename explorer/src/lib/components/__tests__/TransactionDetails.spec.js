import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";

import { apiMarketData, gqlTransaction } from "$lib/mock-data";
import { transformTransaction } from "$lib/chain-info";

import { TransactionDetails } from "..";

describe("Transaction Details", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2024, 4, 20));

  const baseProps = {
    data: transformTransaction(gqlTransaction.tx),
    error: null,
    loading: false,
    market: {
      currentPrice: apiMarketData.market_data.current_price,
      marketCap: apiMarketData.market_data.market_cap,
    },
  };

  afterEach(cleanup);

  afterAll(() => {
    vi.useRealTimers();
  });

  it("renders the Transaction Details component", () => {
    const { container } = render(TransactionDetails, baseProps);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("renders the Transaction Details component with the memo decoded", async () => {
    const { container, getAllByRole } = render(TransactionDetails, baseProps);

    await fireEvent.click(getAllByRole("switch")[0]);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("renders the Transaction Details component with the payload visible", async () => {
    const { container, getAllByRole } = render(TransactionDetails, baseProps);

    await fireEvent.click(getAllByRole("switch")[1]);

    expect(container.firstChild).toMatchSnapshot();
  });
});
