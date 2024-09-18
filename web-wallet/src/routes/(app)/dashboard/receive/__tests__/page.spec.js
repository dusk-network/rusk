import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import Receive from "../+page.svelte";

vi.useFakeTimers();

vi.mock("$lib/dusk/string", async (importOriginal) => {
  /** @type {typeof import("$lib/dusk/string")} */
  const original = await importOriginal();

  return {
    ...original,
    randomUUID: () => "some-generated-id",
  };
});

describe("Receive", () => {
  afterEach(cleanup);

  afterAll(() => {
    vi.doUnmock("$lib/dusk/string");
  });

  it("should render the receive page", async () => {
    const { container } = render(Receive);

    await vi.advanceTimersToNextTimerAsync();

    expect(container.firstChild).toMatchSnapshot();
  });
});
