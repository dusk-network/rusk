import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import Transfer from "../+page.svelte";

vi.useFakeTimers();

describe("Transfer", () => {
  afterEach(cleanup);

  it("should render the transfer page", async () => {
    const { container } = render(Transfer);

    expect(container.firstChild).toMatchSnapshot();
  });
});
