import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";

import { Icon } from "..";

describe("Icon", () => {
  const baseProps = {
    path: "M3,3H21V21H3V3M5,5V19H19V5H5Z",
  };
  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(cleanup);

  it("should render the Icon component", () => {
    const { container } = render(Icon, baseOptions);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should accept a custom role for the SVG component", () => {
    const props = {
      ...baseProps,
      role: "presentation",
    };
    const { container } = render(Icon, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should render the icon inside a `g` element if it's part of a stack", () => {
    const props = {
      ...baseProps,
      isInStack: true,
    };
    const { container } = render(Icon, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should pass additional class names and attributes to the rendered element", () => {
    const props = {
      ...baseProps,
      className: "foo bar",
      "data-baz": "baz",
    };
    const { container, rerender } = render(Icon, { ...baseOptions, props });
    const icon = container.firstChild;

    expect(icon).toHaveClass("foo bar");
    expect(icon).toHaveAttribute("data-baz", "baz");
    expect(icon).toMatchSnapshot();

    rerender({ ...props, isInStack: true });

    const icon2 = container.firstChild;

    expect(icon2).toHaveClass("foo bar");
    expect(icon2).toHaveAttribute("data-baz", "baz");
    expect(icon2).toMatchSnapshot();
  });
});
