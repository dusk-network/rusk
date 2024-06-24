import { afterEach, describe, expect, it } from "vitest";
import { cleanup } from "@testing-library/svelte";

import { renderWithSimpleContent } from "$lib/dusk/test-helpers";

import { Card } from "..";

describe("Card", () => {
  afterEach(cleanup);

  it("renders the Card component", () => {
    const { container } = renderWithSimpleContent(Card, {});

    expect(container.firstChild).toMatchSnapshot();
  });
});
