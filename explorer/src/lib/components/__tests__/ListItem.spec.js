import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { ListItem } from "../";

describe("List Item", () => {
  afterEach(cleanup);

  it("renders the List Item component", () => {
    const { container } = render(ListItem);

    expect(container.firstChild).toMatchSnapshot();
  });
});
