import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { apiTransactions } from "$lib/mock-data";
import { transformTransaction } from "$lib/chain-info";
import { TransactionsTable } from "..";
import { mapWith, slice } from "lamb";

const transformTransactions = mapWith(transformTransaction);
const data = slice(transformTransactions(apiTransactions.data), 0, 10);

describe("Transactions Table", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2024, 4, 20));
  const baseProps = {
    data: data,
  };
  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(cleanup);

  afterAll(() => {
    vi.useRealTimers();
  });

  it("should render the `TransactionsTable` component", () => {
    const { container } = render(TransactionsTable, baseOptions);

    expect(container.firstChild).toMatchSnapshot();
  });
});
