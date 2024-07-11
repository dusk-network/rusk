import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { TransactionsList } from "..";
import { gqlTransaction } from "$lib/mock-data";
import { transformTransaction } from "$lib/chain-info";

describe("Transactions List", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2024, 4, 20));

  /** @type {import("svelte").ComponentProps<TransactionsList>} */
  const baseProps = {
    data: transformTransaction(gqlTransaction.tx),
    mode: "full",
  };

  afterEach(cleanup);

  afterAll(() => {
    vi.useRealTimers();
  });

  it('should render the `TransactionsList` component in "full" mode', () => {
    const { container } = render(TransactionsList, baseProps);

    expect(container.firstChild).toMatchSnapshot();
  });

  it('should render the `TransactionsList` component in "compact" mode', () => {
    /** @type {import("svelte").ComponentProps<TransactionsList>} */
    const props = {
      ...baseProps,
      mode: "compact",
    };
    const { container } = render(TransactionsList, { ...baseProps, ...props });

    expect(container.firstChild).toMatchSnapshot();
  });
});
