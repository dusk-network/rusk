import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { deductLuxFeeFrom } from "$lib/contracts";
import { createCurrencyFormatter } from "$lib/dusk/currency";

import { Stake } from "..";

/**
 * @param {HTMLElement} input
 * @param {*} value
 * @returns {Promise<boolean>}
 */
const fireInput = (input, value) =>
  fireEvent.input(input, { target: { value } });

describe("Stake", () => {
  const formatter = createCurrencyFormatter("en", "DUSK", 9);
  const lastTxId = "some-id";

  const baseProps = {
    execute: vi.fn().mockResolvedValue(lastTxId),

    /** @type {StakeType} */
    flow: "stake",
    formatter,
    gasLimits: {
      gasLimitLower: 10000000,
      gasLimitUpper: 2900000000,
      gasPriceLower: 1,
    },
    gasSettings: {
      gasLimit: 20000000,
      gasPrice: 1,
    },
    hideStakingNotice: true,
    minAllowedStake: 1234,
    rewards: 345,
    spendable: 10000,
    staked: 278,
    statuses: [
      {
        label: "Spendable",
        value: "1,000.000000000",
      },
      {
        label: "Total Locked",
        value: "278.000000000",
      },
      {
        label: "Rewards",
        value: "345.000000000",
      },
    ],
  };
  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  const maxSpendable = deductLuxFeeFrom(
    baseProps.spendable,
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
      baseProps.minAllowedStake.toString()
    );
    expect(amountInput.getAttribute("max")).toBe(maxSpendable.toString());
    expect(container.firstChild).toMatchSnapshot();
  });

  it("should disable the next button if the stake amount is invalid on mount", async () => {
    const props = {
      ...baseProps,
      gasSettings: {
        ...baseProps.gasSettings,
        gasLimit: 40000000,
        gasPrice: 40000000,
      },
    };
    const currentMaxSpendable = deductLuxFeeFrom(
      props.spendable,
      props.gasSettings.gasPrice * props.gasSettings.gasLimit
    );
    const { getByRole } = render(Stake, { ...baseOptions, props });
    const nextButton = getByRole("button", { name: "Next" });
    const amountInput = getByRole("spinbutton");

    await fireInput(amountInput, 1000);
    expect(nextButton).toBeDisabled();
    expect(amountInput.getAttribute("min")).toBe(
      baseProps.minAllowedStake.toString()
    );
    expect(amountInput.getAttribute("max")).toBe(
      currentMaxSpendable.toString()
    );
  });

  it("should set the max amount in the textbox if the user clicks the related button", async () => {
    const { getByRole } = render(Stake, baseOptions);
    const useMaxButton = getByRole("button", { name: "USE MAX" });

    await fireEvent.click(useMaxButton);

    const amountInput = getByRole("spinbutton");

    expect(amountInput).toHaveValue(maxSpendable);
  });

  it("should disable the next button if the user enters an invalid amount", async () => {
    const { getByRole } = render(Stake, baseOptions);
    const nextButton = getByRole("button", { name: "Next" });
    const amountInput = getByRole("spinbutton");

    expect(nextButton).toBeEnabled();

    await fireEvent.input(amountInput, {
      target: { value: baseProps.spendable * 2 },
    });

    expect(nextButton).toBeDisabled();
  });

  it("should render the review step of the Stake component", async () => {
    const { container, getByRole } = render(Stake, baseOptions);

    await fireEvent.click(getByRole("button", { name: "Next" }));

    expect(container.firstChild).toMatchSnapshot();
  });

  describe("Stake operation", () => {
    vi.useFakeTimers();

    const expectedExplorerLink = `https://explorer.dusk.network/transactions/transaction?id=${lastTxId}`;

    afterAll(() => {
      vi.useRealTimers();
    });

    it("should perform a stake for the desired amount, give a success message and supply a link to see the transaction in the explorer", async () => {
      const { getByRole, getByText } = render(Stake, baseProps);
      const amountInput = getByRole("spinbutton");

      expect(amountInput).toHaveValue(baseProps.minAllowedStake);

      await fireEvent.click(getByRole("button", { name: "Next" }));
      await fireEvent.click(getByRole("button", { name: "Stake" }));

      await vi.advanceTimersToNextTimerAsync();

      expect(baseProps.execute).toHaveBeenCalledTimes(1);
      expect(baseProps.execute).toHaveBeenCalledWith(
        baseProps.minAllowedStake,
        baseProps.gasSettings.gasPrice,
        baseProps.gasSettings.gasLimit
      );

      const explorerLink = getByRole("link", { name: /explorer/i });

      expect(getByText("Transaction completed")).toBeInTheDocument();
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
        2567,
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
        maxSpendable,
        baseProps.gasSettings.gasPrice,
        baseProps.gasSettings.gasLimit
      );
      expect(getByText("Transaction completed")).toBeInTheDocument();
      expect(() => getByRole("link", { name: /explorer/i })).toThrow();
    });
  });
});
