import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { Card } from "..";

describe("Card", () => {
  afterEach(cleanup);

  it("renders the Card component", () => {
    const { container } = render(Card);

    expect(container.firstChild).toMatchSnapshot();
  });
});
