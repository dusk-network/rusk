import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { BigIntInput } from "..";

describe("BigIntInput", () => {
  afterEach(cleanup);

  it("initializes with the correct value", () => {
    const { getByDisplayValue } = render(BigIntInput, { value: 123n });
    const input = getByDisplayValue("123");

    expect(input).toBeInTheDocument();
  });

  it("changes the value on valid input", async () => {
    const { getByDisplayValue, component } = render(BigIntInput, {
      value: 123n,
    });
    const input = getByDisplayValue("123");

    let updatedValue;
    component.$on("change", (event) => {
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

  it("emits an error event when the input is a valid number that can be converted to a BigInt but exceed the limits", async () => {
    const errorHandler = vi.fn();
    const { getByDisplayValue, component } = render(BigIntInput, {
      maxValue: 10n,
      minValue: 1n,
      value: 2n,
    });
    const input = getByDisplayValue("2");

    component.$on("error", errorHandler);

    await fireEvent.input(input, { target: { value: "11" } });

    expect(input).toHaveValue("11");
    expect(errorHandler).toHaveBeenCalledTimes(1);
    expect(errorHandler).toHaveBeenCalledWith(expect.any(Event));

    await fireEvent.input(input, { target: { value: "0" } });

    expect(input).toHaveValue("0");
    expect(errorHandler).toHaveBeenCalledTimes(2);
    expect(errorHandler).toHaveBeenCalledWith(expect.any(Event));
  });
});
