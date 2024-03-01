import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";

import { ErrorDetails } from "..";

describe("ErrorDetails", () => {
  const baseProps = {
    error: new Error("Some error messaage"),
    summary: "Some error summary",
  };

  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(cleanup);

  it("should render the `ErrorDetails` component", () => {
    const { container } = render(ErrorDetails, baseOptions);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should pass additional class names to the rendered element", () => {
    const props = {
      ...baseProps,
      className: "foo bar",
    };
    const { container } = render(ErrorDetails, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should render nothing if the error is `null`", () => {
    const props = {
      ...baseProps,
      error: null,
    };
    const { container } = render(ErrorDetails, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });
});
