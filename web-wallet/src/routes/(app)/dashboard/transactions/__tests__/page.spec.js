import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import Transactions from "../+page.svelte";

global.ResizeObserver = vi.fn().mockImplementation(() => ({
  disconnect: vi.fn(),
  observe: vi.fn(),
  unobserve: vi.fn(),
}));

vi.useFakeTimers();

describe("Dashboard", () => {
  afterEach(cleanup);

  const currentPrice = Promise.resolve({ usd: 0.5 });

  it("should render the transactions page", async () => {
    const { container } = render(Transactions, { data: { currentPrice } });

    await vi.advanceTimersToNextTimerAsync();

    expect(container.firstChild).toMatchSnapshot();
  });
});
