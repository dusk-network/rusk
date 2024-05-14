import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";

import { renderWithSimpleContent } from "$lib/dusk/test-helpers";

import { DataGuard } from "..";

describe("DataGuard", () => {
  const baseProps = {
    data: null,
  };
  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(cleanup);

  it("should render the `DataGuard` with the placeholder if no data is passed", () => {
    const { container } = render(DataGuard, baseOptions);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should render the `DataGuard` if data is present", () => {
    const data = 1;
    const renderWithSlots = renderWithSimpleContent(DataGuard, {
      ...baseOptions,
      props: { ...baseProps, data },
    });

    expect(renderWithSlots.container.firstChild).toMatchSnapshot();
  });
});
