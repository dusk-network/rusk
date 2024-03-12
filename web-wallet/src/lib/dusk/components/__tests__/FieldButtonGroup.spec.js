import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { afterEach, describe, expect, it, vi } from "vitest";

import { FieldButtonGroup } from "..";

describe("FieldButtonGroup", () => {
  const baseProps = { name: "test" };
  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(cleanup);

  it("renders the component with default values", async () => {
    const { getByRole, container } = render(FieldButtonGroup, baseOptions);

    const input = getByRole("textbox");
    const button = getByRole("button");

    expect(input).toBeInTheDocument();
    expect(button).toBeInTheDocument();

    expect(container.firstChild).toMatchSnapshot();
  });

  it("updates input value on change", async () => {
    const { getByRole, container } = render(FieldButtonGroup, baseOptions);
    const input = getByRole("textbox");

    await fireEvent.input(input, { target: { value: "test value" } });

    expect(input).toHaveValue("test value");

    expect(container.firstChild).toMatchSnapshot();
  });

  it("triggers click event on button click", async () => {
    const { getByRole, component } = render(FieldButtonGroup);
    const button = getByRole("button");

    const mockClickHandler = vi.fn();
    component.$on("click", mockClickHandler);

    await fireEvent.click(button);

    expect(mockClickHandler).toHaveBeenCalled();
  });

  it("focuses input on focus() call", async () => {
    const { getByRole, component } = render(FieldButtonGroup);
    const input = getByRole("textbox");

    component.focus();
    expect(input).toHaveFocus();
  });

  it("should expose a method to select the element's text", () => {
    const { component, getByRole } = render(FieldButtonGroup, {
      ...baseProps,
      value: "some input text",
    });

    const input = /** @type {HTMLInputElement} */ (getByRole("textbox"));

    component.select();

    const selectedText = input.value.substring(
      Number(input.selectionStart),
      Number(input.selectionEnd)
    );

    expect(selectedText).toBe("some input text");
  });
});
