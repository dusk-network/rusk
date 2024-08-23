import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import Staking from "../+page.svelte";

vi.useFakeTimers();

describe("Staking", () => {
  afterEach(cleanup);

  it("should render the staking page", async () => {
    const { container } = render(Staking);

    await vi.advanceTimersToNextTimerAsync(); // Wait until the stakeInfo promise has resolved

    expect(container.firstChild).toMatchSnapshot();
  });
});
