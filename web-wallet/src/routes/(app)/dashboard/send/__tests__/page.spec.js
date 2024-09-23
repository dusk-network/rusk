import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import Send from "../+page.svelte";

vi.mock("$lib/dusk/string", async (importOriginal) => {
  /** @type {typeof import("$lib/dusk/string")} */
  const original = await importOriginal();

  return {
    ...original,
    randomUUID: () => "some-generated-id",
  };
});

vi.useFakeTimers();

describe("Send", () => {
  afterEach(cleanup);

  it("should render the send page", async () => {
    const { container } = render(Send);

    await vi.advanceTimersToNextTimerAsync();

    expect(container.firstChild).toMatchSnapshot();
  });
});
