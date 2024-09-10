import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import Receive from "../+page.svelte";

vi.useFakeTimers();

describe("Receive", () => {
  afterEach(cleanup);

  it("should render the receive page", async () => {
    const { container } = render(Receive);

    await vi.advanceTimersToNextTimerAsync();

    expect(container.firstChild).toMatchSnapshot();
  });
});
