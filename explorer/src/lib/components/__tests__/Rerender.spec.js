import { afterEach, describe, expect, it } from "vitest";
import { cleanup } from "@testing-library/svelte";
import { Rerender } from "..";

import { renderWithSimpleContent } from "$lib/dusk/test-helpers";

describe("updates the slot content every second", () => {
  const baseOptions = {
    target: document.body,
  };

  afterEach(cleanup);
  it("should render with slot data", () => {
    const renderWithSlots = renderWithSimpleContent(Rerender, baseOptions);

    expect(renderWithSlots.container.firstChild).toMatchSnapshot();
  });
});
