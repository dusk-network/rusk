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
    payload:
      '{"type":"moonlight","sender":"214twXJifCt8TeGRFaxcAcb1HUScSuGa9K5vLHZ26Xyb9iTUCesPjH4YCiMN2tzHQeYuB6e2HEtNvXitaWqP68NiV71wrfNPft4ExcoKzR29LduJb3iM3kQNnMrFS8aw197F","receiver":"214twXJifCt8TeGRFaxcAcb1HUScSuGa9K5vLHZ26Xyb9iTUCesPjH4YCiMN2tzHQeYuB6e2HEtNvXitaWqP68NiV71wrfNPft4ExcoKzR29LduJb3iM3kQNnMrFS8aw197F","value":0,"deposit":0,"fee":{"gas_price":"37","gas_limit":"500000000"},"call":{"fn_name":"reset","contract":"1ea3e990304333fa98e5e31e48a1aaa1506235d8a243ea2168422e56f6681da8","fn_args":"Hg=="},"is_deploy":false,"memo":null}',
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
