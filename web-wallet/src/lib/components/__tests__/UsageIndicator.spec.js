import { afterEach, describe, expect, it } from "vitest";
import { cleanup, getAllByRole, render } from "@testing-library/svelte";

import { UsageIndicator } from "..";

const getPercentages = () =>
  getAllByRole(document.body, "graphics-symbol").map((el) =>
    el
      .getAttribute("data-tooltip-text")
      ?.replace(/^.*?(\d+(\.\d+)?)%.*?$/, "$1")
  );

describe("UsageIndicator", () => {
  const baseProps = {
    value: 45,
  };
  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(cleanup);

  it("should render the `UsageIndicator` component and react to value changes", async () => {
    const { container, rerender } = render(UsageIndicator, baseOptions);

    expect(getPercentages()).toStrictEqual(["45", "55"]);
    expect(container.firstChild).toMatchSnapshot();

    await rerender({ value: 80 });

    expect(getPercentages()).toStrictEqual(["80", "20"]);
    expect(container.firstChild).toMatchSnapshot();
  });

  it("should accept additional class names", () => {
    const props = { ...baseProps, className: "foo bar" };
    const { container } = render(UsageIndicator, { ...baseOptions, props });

    expect(container.firstChild).toHaveClass("usage-indicator foo bar");
  });

  it("should round decimal values at two decimals", async () => {
    const props = { ...baseProps, value: 45.7687 };
    const { rerender } = render(UsageIndicator, { ...baseOptions, props });

    expect(getPercentages()).toStrictEqual(["45.77", "54.23"]);

    await rerender({ value: 0.005 });

    expect(getPercentages()).toStrictEqual(["0.01", "99.99"]);
  });
});
