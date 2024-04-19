import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import Transaction from "../+page.svelte";

global.ResizeObserver = vi.fn().mockImplementation(() => ({
  disconnect: vi.fn(),
  observe: vi.fn(),
  unobserve: vi.fn(),
}));

describe("Transaction", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2024, 4, 15));

  afterEach(() => {
    cleanup();
  });

  afterAll(() => {
    vi.useRealTimers();
  });

  it("should render the Transaction page", () => {
    const { container } = render(Transaction, {});

    expect(container.firstChild).toMatchSnapshot();
  });
});
