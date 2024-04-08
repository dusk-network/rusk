import { afterEach, describe, expect, it } from "vitest";
import { cleanup } from "@testing-library/svelte";

import { renderWithSimpleContent } from "$lib/dusk/test-helpers";

import { Anchor } from "..";

describe("Anchor", () => {
  const baseProps = {
    href: "https://example.com",
  };

  const baseOptions = {
    props: baseProps,
  };

  afterEach(cleanup);

  it("should render the Anchor component", () => {
    const { container } = renderWithSimpleContent(Anchor, baseOptions);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should pass additional class names and attributes to the rendered element", () => {
    const props = {
      ...baseProps,
      className: "foo bar",
      href: "https://dusk.network",
      title: "Some title",
    };
    const { container } = renderWithSimpleContent(Anchor, {
      ...baseOptions,
      props,
    });

    expect(container.firstChild).toMatchSnapshot();
  });
});
