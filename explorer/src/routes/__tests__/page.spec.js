import { afterAll, describe, expect, it, vi } from "vitest";
import { render } from "@testing-library/svelte";
import ChainInfo from "../+page.svelte";

vi.useFakeTimers();
vi.setSystemTime(new Date(2024, 4, 15));

describe("Chain Info", () => {
  afterAll(() => {
    vi.useRealTimers();
  });

  it("should render the Chain Info page", () => {
    const { container } = render(ChainInfo, {});

    expect(container.firstChild).toMatchSnapshot();
  });
});
