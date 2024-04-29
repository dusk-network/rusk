import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { TransactionsList } from "..";
import { apiTransaction } from "$lib/mock-data";
import { transformTransaction } from "$lib/chain-info";

global.ResizeObserver = vi.fn().mockImplementation(() => ({
  disconnect: vi.fn(),
  observe: vi.fn(),
  unobserve: vi.fn(),
}));

const baseProps = { data: transformTransaction(apiTransaction.data[0]) };

describe("Transactions List", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2024, 4, 20));

  afterEach(cleanup);

  afterAll(() => {
    vi.useRealTimers();
  });

  it("renders the Transactions List component", () => {
    const { container } = render(TransactionsList, baseProps);

    expect(container.firstChild).toMatchSnapshot();
  });
});
