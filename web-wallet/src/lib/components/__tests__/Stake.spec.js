import {
  afterAll,
  afterEach,
  beforeAll,
  describe,
  expect,
  it,
  vi,
} from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import {
  createCurrencyFormatter,
  duskToLux,
  luxToDusk,
} from "$lib/dusk/currency";

import { Stake } from "..";

/**
 * @param {HTMLElement} input
 * @param {*} value
 * @returns {Promise<boolean>}
 */
const fireInput = (input, value) =>
  fireEvent.input(input, { target: { value } });

vi.mock("$lib/dusk/string", async (importOriginal) => {
  /** @type {typeof import("$lib/dusk/string")} */
  const original = await importOriginal();

  return {
    ...original,
    randomUUID: () => "some-generated-id",
  };
});

describe("Stake", () => {
  const formatter = createCurrencyFormatter("en", "DUSK", 9);
  const lastTxId = "some-id";

  const baseProps = {
    availableBalance: 10_000_000_000_000n,
    execute: vi.fn().mockResolvedValue(lastTxId),
    formatter,
    gasLimits: {
      gasLimitLower: 10000000n,
      gasLimitUpper: 2900000000n,
      gasPriceLower: 1n,
    },
    gasSettings: {
      gasLimit: 20000000n,
      gasPrice: 1n,
    },
    hideStakingNotice: true,
    minStakeRequirement: 1_234_000_000_000n,
  };
  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  const maxSpendableDusk = luxToDusk(
    baseProps.availableBalance -
      baseProps.gasSettings.gasPrice * baseProps.gasSettings.gasLimit
  );

  afterEach(() => {
    cleanup();
    baseProps.execute.mockClear();
  });

  it("should render the Stake notice", () => {
    const options = {
      ...baseOptions.target,
      props: {
        ...baseProps,
        hideStakingNotice: false,
      },
    };

    vi.spyOn(Math, "random").mockReturnValue(42);

    const { container } = render(Stake, options);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should render the Stake component", () => {
    const { container, getByRole } = render(Stake, baseOptions);
    const nextButton = getByRole("button", { name: "Next" });
    const amountInput = getByRole("spinbutton");

    expect(nextButton).toBeEnabled();
    expect(amountInput.getAttribute("min")).toBe(
      luxToDusk(baseProps.minStakeRequirement).toString()
    );
    expect(amountInput.getAttribute("max")).toBe(maxSpendableDusk.toString());
    expect(container.firstChild).toMatchSnapshot();
  });

  it("should disable the next button if the stake amount is invalid on mount", async () => {
    const props = {
      ...baseProps,
      gasSettings: {
        ...baseProps.gasSettings,
        gasLimit: 40000000n,
        gasPrice: 40000000n,
      },
    };
    const { getByRole } = render(Stake, { ...baseOptions, props });
    const nextButton = getByRole("button", { name: "Next" });
    const amountInput = getByRole("spinbutton");

    expect(nextButton).toBeDisabled();
    await fireInput(amountInput, 1000);
    expect(nextButton).toBeDisabled();
  });

  it("should set the max amount in the textbox if the user clicks the related button", async () => {
    const { getByRole } = render(Stake, baseOptions);
    const useMaxButton = getByRole("button", { name: "USE MAX" });

    await fireEvent.click(useMaxButton);

    const amountInput = getByRole("spinbutton");

    expect(amountInput).toHaveValue(maxSpendableDusk);
  });

  it("should not change the default amount (min stake amount) in the textbox if the user clicks the related button and the balance is zero", async () => {
    const props = {
      ...baseProps,
      availableBalance: 0n,
    };

    const { getByRole } = render(Stake, props);

    const useMaxButton = getByRole("button", { name: "USE MAX" });
    const amountInput = getByRole("spinbutton");

    expect(amountInput).toHaveValue(luxToDusk(baseProps.minStakeRequirement));

    await fireEvent.click(useMaxButton);

    expect(amountInput).toHaveValue(luxToDusk(baseProps.minStakeRequirement));
  });

  it("should not change the default amount (1) in the textbox if the user clicks the related button and the gas settings are invalid", async () => {
    const props = {
      ...baseProps,
      gasSettings: {
        ...baseProps.gasSettings,
        gasLimit: 40000000n,
        gasPrice: 40000000n,
      },
    };

    const { getByRole } = render(Stake, props);
    const useMaxButton = getByRole("button", { name: "USE MAX" });
    const amountInput = getByRole("spinbutton");

    expect(amountInput).toHaveValue(luxToDusk(baseProps.minStakeRequirement));

    await fireEvent.click(useMaxButton);

    expect(amountInput).toHaveValue(luxToDusk(baseProps.minStakeRequirement));
  });

  it("should disable the next button if the user enters an invalid amount", async () => {
    const { getByRole } = render(Stake, baseOptions);
    const nextButton = getByRole("button", { name: "Next" });
    const amountInput = getByRole("spinbutton");

    expect(nextButton).toBeEnabled();

    await fireEvent.input(amountInput, {
      target: { value: luxToDusk(baseProps.availableBalance) * 2 },
    });

    expect(nextButton).toBeDisabled();
  });

  it("should render the review step of the Stake component", async () => {
    const { container, getByRole } = render(Stake, baseOptions);

    await fireEvent.click(getByRole("button", { name: "Next" }));

    expect(container.firstChild).toMatchSnapshot();
  });

  describe("Stake operation", () => {
    beforeAll(() => {
      vi.useFakeTimers();
    });

    afterAll(() => {
      vi.useRealTimers();
    });

    const expectedExplorerLink = `/explorer/transactions/transaction?id=${lastTxId}`;

    it("should perform a stake for the desired amount, give a success message and supply a link to see the transaction in the explorer", async () => {
      const { getByRole, getByText } = render(Stake, baseProps);
      const amountInput = getByRole("spinbutton");

      expect(amountInput).toHaveValue(luxToDusk(baseProps.minStakeRequirement));

      await fireEvent.click(getByRole("button", { name: "Next" }));
      await fireEvent.click(getByRole("button", { name: "Stake" }));

      await vi.advanceTimersToNextTimerAsync();

      expect(baseProps.execute).toHaveBeenCalledTimes(1);
      expect(baseProps.execute).toHaveBeenCalledWith(
        baseProps.minStakeRequirement,
        baseProps.gasSettings.gasPrice,
        baseProps.gasSettings.gasLimit
      );

      const explorerLink = getByRole("link", { name: /explorer/i });

      expect(getByText("Transaction created")).toBeInTheDocument();
      expect(explorerLink).toHaveAttribute("target", "_blank");
      expect(explorerLink).toHaveAttribute("href", expectedExplorerLink);
    });

    it("should show an error message if the transfer fails", async () => {
      const errorMessage = "Some error message";

      baseProps.execute.mockRejectedValueOnce(new Error(errorMessage));

      const { getByRole, getByText } = render(Stake, baseProps);
      const amountInput = getByRole("spinbutton");

      await fireEvent.input(amountInput, { target: { value: 2567 } });
      await fireEvent.click(getByRole("button", { name: "Next" }));
      await fireEvent.click(getByRole("button", { name: "Stake" }));

      await vi.advanceTimersToNextTimerAsync();

      expect(baseProps.execute).toHaveBeenCalledTimes(1);
      expect(baseProps.execute).toHaveBeenCalledWith(
        duskToLux(2567),
        baseProps.gasSettings.gasPrice,
        baseProps.gasSettings.gasLimit
      );
      expect(getByText("Transaction failed")).toBeInTheDocument();
      expect(getByText(errorMessage)).toBeInTheDocument();
    });

    it("should show the success message, but no explorer link, if the execution promise doesn't resolve with an hash", async () => {
      baseProps.execute.mockResolvedValueOnce(void 0);

      const { getByRole, getByText } = render(Stake, baseProps);

      await fireEvent.click(getByRole("button", { name: "USE MAX" }));
      await fireEvent.click(getByRole("button", { name: "Next" }));
      await fireEvent.click(getByRole("button", { name: "Stake" }));

      await vi.advanceTimersToNextTimerAsync();

      expect(baseProps.execute).toHaveBeenCalledTimes(1);
      expect(baseProps.execute).toHaveBeenCalledWith(
        duskToLux(maxSpendableDusk),
        baseProps.gasSettings.gasPrice,
        baseProps.gasSettings.gasLimit
      );
      expect(getByText("Transaction created")).toBeInTheDocument();
      expect(() => getByRole("link", { name: /explorer/i })).toThrow();
    });
  });
});
