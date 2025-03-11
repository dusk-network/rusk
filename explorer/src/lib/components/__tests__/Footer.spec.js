import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { Footer } from "../";

describe("Footer", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2025, 0, 1));
  });

  afterEach(() => {
    cleanup();
    vi.useRealTimers();
  });

  it("renders the Footer component", () => {
    const { container } = render(Footer);

    expect(container.firstChild).toMatchSnapshot();
  });
});
