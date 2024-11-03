import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";

import { GasSettings } from "..";
import { get } from "svelte/store";
import { settingsStore } from "$lib/stores";
import { createCurrencyFormatter } from "$lib/dusk/currency";

describe("GasSettings", () => {
  const settings = get(settingsStore);
  const duskFormatter = createCurrencyFormatter(settings.language, "DUSK", 9);
  const fee = settings.gasPrice * settings.gasLimit;

  const baseProps = {
    fee: fee,
    formatter: duskFormatter,
    limit: 20000000n,
    limitLower: 10000000n,
    limitUpper: 1000000000n,
    price: 1n,
    priceLower: 1n,
  };

  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(cleanup);

  it("renders the GasSettings component closed", () => {
    const { container } = render(GasSettings, baseOptions);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("renders the GasSettings component opened", async () => {
    const { container, getByRole } = render(GasSettings, baseOptions);

    const next = getByRole("button", { name: "EDIT" });

    await fireEvent.click(next);

    expect(container.firstChild).toMatchSnapshot();
  });

  it('checks "gasSettings" event is dispatched on click with the correct event data', async () => {
    const eventHandler = vi.fn();
    const { component, getByRole, getAllByRole } = render(
      GasSettings,
      baseOptions
    );
    const editButton = getByRole("button", { name: "EDIT" });

    expect(() => getAllByRole("textbox")).toThrow();

    await fireEvent.click(editButton);

    component.$on("gasSettings", eventHandler);

    const [priceInput, limitInput] = getAllByRole("textbox");

    await fireEvent.input(limitInput, {
      target: { value: baseProps.limitLower },
    });

    expect(eventHandler).toHaveBeenCalledTimes(1);
    expect(eventHandler.mock.lastCall?.[0].detail).toStrictEqual({
      limit: baseProps.limitLower,
      price: baseProps.price,
    });

    await fireEvent.input(priceInput, {
      target: { value: baseProps.price * 2n },
    });

    expect(eventHandler).toHaveBeenCalledTimes(2);
    expect(eventHandler.mock.lastCall?.[0].detail).toStrictEqual({
      limit: baseProps.limitLower,
      price: baseProps.price * 2n,
    });
  });
});
