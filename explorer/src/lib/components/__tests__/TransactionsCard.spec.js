import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { compose, mapWith, take } from "lamb";

import { gqlTransactions } from "$lib/mock-data";
import { transformTransaction } from "$lib/chain-info";

import { TransactionsCard } from "..";

describe("Transactions Card", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2024, 4, 30));

  const getTenTransactions = compose(mapWith(transformTransaction), take(10));
  const data = getTenTransactions(gqlTransactions.transactions);

  const baseProps = {
    error: null,
    isSmallScreen: false,
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

    expect(getByRole("button", { name: "Show More" })).toBeDisabled();

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should disable the `Show More` button if there is no more data to display", async () => {
    const loading = false;
    const txns = data;

    const { container, getByRole } = render(TransactionsCard, {
      ...baseOptions,
      props: { ...baseProps, loading, txns },
    });

    expect(getByRole("button", { name: "Show More" })).toBeDisabled();

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should hide the `Show More` button if an error has occurred", async () => {
    const props = { ...baseProps, error: new Error("error") };

    const { container } = render(TransactionsCard, {
      ...baseOptions,
      props: { ...props, error: new Error("error") },
    });

    expect(container.firstChild).toMatchSnapshot();
  });
});
