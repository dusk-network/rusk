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

  it("should be able to render the details when they contain blob hashes", () => {
    const props = {
      ...baseProps,
      data: {
        ...baseProps.data,
        blobHashes: [
          "0261047715f0e937f3ab3d6bdfb1bf1894995f89f64ed19a26a1d59bb2d7b629",
          "b3d5296139ba0f44912b87a19b47ea7f229131182405d9f082c5fbbeed8c121b",
          "8f6bce4e1f233d6de022e3ac1ab3a262695991460e9f78eca288fd623083142f",
        ],
      },
    };

    const { container } = render(TransactionDetails, props);

    expect(container.firstChild).toMatchSnapshot();
  });
});
