import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { GasControls } from "..";

/**
 * @param {HTMLElement} input
 * @param {*} value
 * @returns {Promise<boolean>}
 */
const fireInput = (input, value) =>
  fireEvent.input(input, { target: { value } });

/** @param {HTMLElement} element */
function asInput(element) {
  // eslint-disable-next-line no-extra-parens
  return /** @type {HTMLInputElement} */ (element);
}

describe("GasControls", () => {
  const baseProps = {
    limit: 20n,
    limitLower: 10n,
    limitUpper: 100n,
    price: 10n,
    priceLower: 1n,
  };
  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  const eventHandler = vi.fn();

  afterEach(() => {
    cleanup();
    eventHandler.mockClear();
  });

  it("should render the `GasControls` component", () => {
    const { container } = render(GasControls, baseOptions);
    expect(container).toMatchSnapshot();
  });

  it('should dispatch a "gasSettings" event when the price or the limit are changed', async () => {
    const { component, getByLabelText } = render(GasControls, baseOptions);
    const priceInput = asInput(getByLabelText(/price/i));
    const limitInput = asInput(getByLabelText(/limit/i));

    component.$on("gasSettings", eventHandler);

    await fireInput(priceInput, 15);

    expect(eventHandler).toHaveBeenCalledTimes(1);
    expect(eventHandler.mock.lastCall?.[0].detail).toStrictEqual({
      limit: baseProps.limit,
      price: 15n,
    });
    expect(BigInt(priceInput.value)).toBe(15n);

    await fireInput(limitInput, 25);

    expect(eventHandler).toHaveBeenCalledTimes(2);
    expect(eventHandler.mock.lastCall?.[0].detail).toStrictEqual({
      limit: 25n,
      price: 15n,
    });
    expect(BigInt(limitInput.value)).toBe(25n);
  });
});
