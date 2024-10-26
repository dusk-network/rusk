import { afterEach, describe, expect, it } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { BigIntInput } from "..";

describe("BigIntInput", () => {
  afterEach(cleanup);

  it("initializes with the correct value", () => {
    const { getByDisplayValue } = render(BigIntInput, { value: 123n });
    const input = getByDisplayValue("123");

    expect(input).toBeInTheDocument();
  });

  it("updates value on valid input", async () => {
    const { getByDisplayValue, component } = render(BigIntInput, {
      value: 123n,
    });
    const input = getByDisplayValue("123");

    let updatedValue;
    component.$on("update", (event) => {
      updatedValue = event.detail;
    });

    await fireEvent.input(input, { target: { value: "456" } });

    expect(input).toHaveValue("456");
    expect(updatedValue).toBe(456n);
  });

  it("does not accept invalid input", async () => {
    const { getByDisplayValue } = render(BigIntInput, { value: 123n });
    const input = getByDisplayValue("123");

    await fireEvent.input(input, { target: { value: "a" } });

    expect(input).toHaveValue("123");
  });
});
