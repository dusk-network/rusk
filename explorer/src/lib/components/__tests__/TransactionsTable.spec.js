import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { mapWith, slice } from "lamb";

import { gqlTransactions } from "$lib/mock-data";
import { transformTransaction } from "$lib/chain-info";

import { TransactionsTable } from "..";

describe("Transactions Table", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2024, 4, 30));

  const transformTransactions = mapWith(transformTransaction);
  const data = slice(
    transformTransactions(gqlTransactions.transactions),
    0,
    10
  );

  /** @type {import("svelte").ComponentProps<TransactionsTable>} */
  const baseProps = {
    data: data,
    mode: "full",
  };
  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(cleanup);

  afterAll(() => {
    vi.useRealTimers();
  });

  it('should render the `TransactionsTable` component in "full" mode', () => {
    const { container } = render(TransactionsTable, baseOptions);

    expect(container.firstChild).toMatchSnapshot();
  });

  it('should render the `TransactionsTable` component in "compact" mode', () => {
    /** @type {import("svelte").ComponentProps<TransactionsTable>} */
    const props = { ...baseProps, mode: "compact" };
    const { container } = render(TransactionsTable, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should pass additional class names to the rendered element", () => {
    const props = { ...baseProps, className: "foo bar" };
    const { container } = render(TransactionsTable, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });
});
