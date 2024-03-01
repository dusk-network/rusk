import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";

import { Throbber } from "..";

describe("Throbber", () => {
  const baseProps = {};
  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(cleanup);

  it("should render the Throbber component", () => {
    const { container } = render(Throbber, baseOptions);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should pass additional class names to the rendered element", () => {
    const props = {
      ...baseProps,
      className: "foo bar",
    };
    const { container } = render(Throbber, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should accept a custom duration and size", () => {
    const props = {
      ...baseProps,
      duration: 500,
      size: 16,
    };
    const { container } = render(Throbber, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });
});
