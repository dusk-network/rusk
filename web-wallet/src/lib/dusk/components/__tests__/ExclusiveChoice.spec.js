import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";

import { ExclusiveChoice } from "..";

vi.mock("$lib/dusk/string", async (importOriginal) => {
  /** @type {typeof import("$lib/dusk/string")} */
  const original = await importOriginal();

  return {
    ...original,
    randomUUID: () => "some-generated-id",
  };
});

describe("ExclusiveChoice", () => {
  const stringOptions = ["one", "two", "three", "four"];

  /** @type {SelectOption[]} */
  const objectOptionsA = [
    { label: "one", value: "1" },
    { label: "two", value: "2" },
    { disabled: true, label: "three", value: "3" },
    { label: "four", value: "4" },
  ];

  /** @type {SelectOption[]} */
  const objectOptionsB = [
    { value: "1" },
    { value: "2" },
    { value: "3" },
    { value: "4" },
  ];

  const baseProps = {
    options: objectOptionsA,
    value: "2",
  };
  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(cleanup);

  afterAll(() => {
    vi.doUnmock("$lib/dusk/string");
  });

  it("should render the `ExclusiveChoice` component", () => {
    const { container } = render(ExclusiveChoice, baseOptions);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should accept a custom name for the radio elements", () => {
    const props = {
      ...baseProps,
      name: "my-custom-name",
    };
    const { container } = render(ExclusiveChoice, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should accept an array of options object without labels and use the value as labels", () => {
    const props = {
      ...baseProps,
      options: objectOptionsB,
    };
    const { container } = render(ExclusiveChoice, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should accept an array of string as options", () => {
    const props = {
      ...baseProps,
      options: stringOptions,
    };
    const { container } = render(ExclusiveChoice, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should pass additional class names and attributes to the rendered element", () => {
    const props = {
      ...baseProps,
      className: "foo bar",
      id: "some-id",
    };
    const { container } = render(ExclusiveChoice, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should accept a change event handler", async () => {
    const changeHandler = vi.fn();
    const { component, container } = render(ExclusiveChoice, baseOptions);
    const target = /** @type {HTMLInputElement} */ (
      container.querySelector("input[value='4']")
    );

    component.$on("change", changeHandler);

    await fireEvent.click(target);

    expect(changeHandler).toHaveBeenCalledTimes(1);
    expect(changeHandler).toHaveBeenCalledWith(expect.any(Event));
    expect(target.checked).toBe(true);
  });
});
