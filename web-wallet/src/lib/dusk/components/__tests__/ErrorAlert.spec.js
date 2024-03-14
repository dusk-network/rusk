import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";

import { ErrorAlert } from "..";

describe("ErrorAlert", () => {
  const baseProps = {
    error: new Error("some error message"),
    summary: "Some error summary",
  };

  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(cleanup);

  it("should render the `ErrorAlert` component", () => {
    const { container } = render(ErrorAlert, baseOptions);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should pass additional class names and attributes to the rendered element", () => {
    const props = {
      ...baseProps,
      className: "foo bar",
      "data-foo": "baz",
      id: "some-id",
    };
    const { container } = render(ErrorAlert, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });
});
