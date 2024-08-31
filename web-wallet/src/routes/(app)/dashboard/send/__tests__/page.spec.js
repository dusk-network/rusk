import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import Send from "../+page.svelte";

vi.useFakeTimers();

describe("Send", () => {
  afterEach(cleanup);

  it("should render the send page", async () => {
    const { container } = render(Send);

    await vi.advanceTimersToNextTimerAsync();

    expect(container.firstChild).toMatchSnapshot();
  });
});
