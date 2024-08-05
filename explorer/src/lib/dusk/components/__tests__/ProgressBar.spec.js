import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";

import { ProgressBar } from "..";

describe("ProgressBar", () => {
  const baseProps = {
    ariaLabel: "Loading",
    currentPercentage: 0,
  };
  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(cleanup);

  it("renders the `ProgressBar` component with no current percentage set", () => {
    const { container } = render(ProgressBar, {
      ...baseOptions,
      props: { ariaLabel: baseProps.ariaLabel },
    });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("renders the `ProgressBar` component with current percentage set as zero", () => {
    const { container } = render(ProgressBar, baseOptions);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("re-renders the `ProgressBar` component when the current percentage property changes", async () => {
    const { container, rerender } = render(ProgressBar, baseOptions);

    expect(container.firstChild).toMatchSnapshot();

    await rerender({ currentPercentage: 50 });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should pass additional class names to the rendered element", () => {
    const { container } = render(ProgressBar, {
      ...baseOptions,
      props: { ...baseProps, className: "foo bar" },
    });

    expect(container.firstChild).toMatchSnapshot();
  });
});
