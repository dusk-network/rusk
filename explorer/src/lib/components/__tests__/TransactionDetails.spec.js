import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { apiTransaction } from "$lib/mock-data";
import { transformTransaction } from "$lib/chain-info";
import { TransactionDetails } from "../";

global.ResizeObserver = vi.fn().mockImplementation(() => ({
  disconnect: vi.fn(),
  observe: vi.fn(),
  unobserve: vi.fn(),
}));

const baseProps = {
  data: transformTransaction(apiTransaction.data[0]),
  error: null,
  loading: false,
};

describe("Transaction Details", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2024, 4, 20));
  afterEach(cleanup);
  afterAll(() => {
    vi.useRealTimers();
  });

  it("renders the Transaction Details component", () => {
    const { container } = render(TransactionDetails, baseProps);

    expect(container.firstChild).toMatchSnapshot();
  });
});
