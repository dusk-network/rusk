import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { apiTransactions } from "$lib/mock-data";
import { transformTransaction } from "$lib/chain-info";
import { TransactionsCard } from "..";
import { compose, mapWith, take } from "lamb";

global.ResizeObserver = vi.fn().mockImplementation(() => ({
  disconnect: vi.fn(),
  observe: vi.fn(),
  unobserve: vi.fn(),
}));

const getTenTransactions = compose(mapWith(transformTransaction), take(10));
const data = getTenTransactions(apiTransactions.data);

describe("Transactions Card", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2024, 4, 20));
  const baseProps = {
    error: null,
    loading: false,
    txns: null,
  };
  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(cleanup);

  afterAll(() => {
    vi.useRealTimers();
  });

  it("should render the `TransactionsCard` component", () => {
    const { container } = render(TransactionsCard, baseOptions);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should disable the `Show More` button if the card is in the loading state", async () => {
    const loading = true;

    const { container, getByRole } = render(TransactionsCard, {
      ...baseOptions,
      props: { ...baseProps, loading },
    });

    expect(getByRole("button")).toBeDisabled();

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should disable the `Show More` button if there is no more data to display", async () => {
    const loading = false;
    const txns = data;

    const { container, getByRole } = render(TransactionsCard, {
      ...baseOptions,
      props: { ...baseProps, loading, txns },
    });

    expect(getByRole("button")).toBeDisabled();

    expect(container.firstChild).toMatchSnapshot();
  });
});
