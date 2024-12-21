import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { mdiCheckDecagramOutline } from "@mdi/js";

import { Stepper } from "..";

vi.mock("$lib/dusk/string", async (importOriginal) => {
  /** @type {typeof import("$lib/dusk/string")} */
  const original = await importOriginal();

  return {
    ...original,
    randomUUID: () => "some-generated-id",
  };
});

describe("Stepper", () => {
  const baseProps = {
    activeStep: 2,
    steps: [
      { label: "foo" },
      { label: "bar" },
      { label: "baz" },
      { label: "qux" },
      { iconPath: mdiCheckDecagramOutline, label: "quux" },
    ],
  };
  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(cleanup);

  afterAll(() => {
    vi.doUnmock("$lib/dusk/string");
  });

  it("should render the `Stepper` component accepting an array of `StepperStep` objects as steps", async () => {
    const { container, rerender } = render(Stepper, baseOptions);

    expect(container.firstChild).toMatchSnapshot();

    await rerender({ ...baseProps, activeStep: 3 });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should render the `Stepper` component accepting a number as the amount of steps", async () => {
    const props = { ...baseProps, steps: 5 };
    const { container, rerender } = render(Stepper, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();

    await rerender({ ...baseProps, activeStep: 3 });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should render the `Stepper` with all step labels being displayed, if the right flag is provided ", async () => {
    const props = { ...baseProps, showStepLabelWhenInactive: true, steps: 5 };
    const { container } = render(Stepper, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should pass additional class names and attributes to the rendered element", () => {
    const props = {
      ...baseProps,
      className: "foo bar",
      id: "some-id",
    };
    const { container } = render(Stepper, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should add the proper class name for the desired variant", () => {
    const props = {
      ...baseProps,

      /** @type {StepperVariant} */
      variant: "secondary",
    };
    const { container } = render(Stepper, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should allow to hide the step numbers", () => {
    const props = {
      ...baseProps,
      showStepNumbers: false,
    };
    const { container } = render(Stepper, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });
});
