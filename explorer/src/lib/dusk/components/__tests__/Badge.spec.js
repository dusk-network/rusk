import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";

import { Badge } from "..";

describe("Badge", () => {
  const baseProps = {
    text: "Badge",
  };

  afterEach(cleanup);

  it('should render the Badge component using the type "neutral" as a default', () => {
    const { container } = render(Badge, baseProps);

    expect(container.firstChild).toMatchSnapshot();
  });

  it('should render the Badge component using the type "warning" variant', () => {
    const { container } = render(Badge, { ...baseProps, variant: "warning" });

    expect(container.firstChild).toMatchSnapshot();
  });

  it('should render the Badge component using the type "error" variant', () => {
    const { container } = render(Badge, { ...baseProps, variant: "error" });

    expect(container.firstChild).toMatchSnapshot();
  });

  it('should render the Badge component using the type "success" variant', () => {
    const { container } = render(Badge, { ...baseProps, variant: "success" });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should pass additional class names and attributes to the root element", () => {
    const props = {
      ...baseProps,
      className: "foo bar",
      id: "some-id",
    };
    const { container } = render(Badge, { ...props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should only display a string 10 characters long", () => {
    const props = {
      ...baseProps,
      maxlength: 10,
      text: "ABCDEFGHIJK",
    };
    const { container } = render(Badge, { ...props });
    expect(container.firstChild?.textContent?.length).toBe(10);
  });
});
