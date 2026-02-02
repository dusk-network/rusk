import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import Setup from "../+page.svelte";

vi.mock("css-doodle", () => ({}));

describe("Setup", () => {
  afterEach(cleanup);

  afterAll(() => {
    vi.doUnmock("css-doodle");
  });

  it("should render the Setup page", () => {
    const { container } = render(Setup, {});

    expect(container.firstChild).toMatchSnapshot();
  });
});
