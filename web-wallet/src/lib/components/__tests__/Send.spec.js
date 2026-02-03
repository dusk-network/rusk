import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import {
  createCurrencyFormatter,
  duskToLux,
  luxToDusk,
} from "$lib/dusk/currency";
import { getAsHTMLElement } from "$lib/dusk/test-helpers";

import { Send } from "..";
import { tick } from "svelte";

vi.mock("$lib/dusk/string", async (importOriginal) => {
  /** @type {typeof import("$lib/dusk/string")} */
  const original = await importOriginal();

  return {
    ...original,
    randomUUID: () => "some-generated-id",
  };
});

describe("Send", () => {
  const formatter = createCurrencyFormatter("en", "DUSK", 9);
  const lastTxId = "some-id";
  const publicAddress =
    "zTsZq814KfWUAQujzjBchbMEvqA1FiKBUakMCtAc2zCa74h9YVz4a2roYwS7LHDHeBwS1aap4f3GYhQBrxroYgsBcE4FJdkUbvpSD5LVXY6JRXNgMXgk6ckTPJUFKoHybff";
  const shieldedAddress =
    "47jNTgAhzn9KCKF3msCfvKg3k1P1QpPCLZ3HG3AoNp87sQ5WNS3QyjckYHWeuXqW7uvLmbKgejpP8Xkcip89vnMM";
  const bep20BridgeAddress = "0x1234567890123456789012345678901234567890"; // Example BEP20 bridge address
  const baseProps = {
    availableBalance: 1_000_000_000_000n,
    bep20BridgeAddress,
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
    publicAddress,
    shieldedAddress,
  };

  const invalidAddress =
    "aB5rL7yC2zK9eV3xH1gQ6fP4jD8sM0iU2oX7wG9nZ8lT3hU4jP5mK8nS6qJ3wF4aA9bB2cC5dD8eE7";

  afterEach(() => {
    cleanup();
    baseProps.execute.mockClear();
  });

  describe("Address step", () => {
    it("should render the Send component Address step", () => {
      const { container } = render(Send, baseProps);

      expect(container.firstChild).toMatchSnapshot();
    });

    it("should disable the next button if the address is empty", () => {
      const { getByRole } = render(Send, baseProps);
      const nextButton = getByRole("button", { name: "Next" });
      const addressInput = getByRole("textbox");

      expect(addressInput).toHaveValue("");
      expect(nextButton).toBeDisabled();
    });

    it("should disable the next button if the address is invalid", async () => {
      const { getByRole } = render(Send, baseProps);
      const nextButton = getByRole("button", { name: "Next" });
      const addressInput = getByRole("textbox");

      await fireEvent.input(addressInput, {
        target: { value: invalidAddress },
      });

      expect(addressInput).toHaveValue(invalidAddress);
      expect(nextButton).toBeDisabled();
    });

    it("should display a warning if the address input is a public account", async () => {
      const { container, getByRole } = render(Send, baseProps);
      const sendToAddress =
        "aTsZq814KfWUAQujzjBchbMEvqA1FiKBUakMCtAc2zCa74h9YVz4a2roYwS7LHDHeBwS1aap4f3GYhQBrxroYgsBcE5FJdkUbvpSD5LVXY6JRXNgMXgk6ckTPJUFKoHybff";
      const addressInput = getByRole("textbox");

      await fireEvent.input(addressInput, {
        target: { value: sendToAddress },
      });

      expect(addressInput).toHaveValue(sendToAddress);
      expect(container.firstChild).toMatchSnapshot();
    });

    it("should display a warning if the address input is self-referential", async () => {
      const { container, getByRole } = render(Send, baseProps);
      const addressInput = getByRole("textbox");

      await fireEvent.input(addressInput, {
        target: { value: publicAddress },
      });

      expect(addressInput).toHaveValue(publicAddress);
      expect(container.firstChild).toMatchSnapshot();
    });

    it("should display a notice if the address input is the BEP20 bridge address", async () => {
      const { getByRole, findByText } = render(Send, baseProps);
      const addressInput = getByRole("textbox");

      await fireEvent.input(addressInput, {
        target: { value: bep20BridgeAddress },
      });

      expect(addressInput).toHaveValue(bep20BridgeAddress);
      expect(
        await findByText("BEP20 bridge operation detected")
      ).toBeInTheDocument();
    });
  });

  describe("Amount step", () => {
    it("should render the Send component Amount step", async () => {
      const { container, getByRole } = render(Send, baseProps);

      await fireEvent.click(getByRole("button", { name: "Next" }));

      expect(container.firstChild).toMatchSnapshot();
    });

    it("should disable the next button if the amount is invalid on mount", async () => {
      const props = {
        ...baseProps,
        gasSettings: {
          ...baseProps.gasSettings,
          gasLimit: 40000000n,
          gasPrice: 40000000n,
        },
      };
      const { getByRole } = render(Send, props);

      await fireEvent.click(getByRole("button", { name: "Next" }));

      const next = getByRole("button", { name: "Next" });

      await tick();

      expect(next).toBeDisabled();
    });

    it("should set the max amount in the textbox if the user clicks the related button", async () => {
      const maxSpendableDusk = luxToDusk(
        baseProps.availableBalance -
          baseProps.gasSettings.gasPrice * baseProps.gasSettings.gasLimit
      );
      const { getByRole } = render(Send, baseProps);

      await fireEvent.click(getByRole("button", { name: "Next" }));

      const useMaxButton = getByRole("button", { name: "USE MAX" });
      const nextButton = getByRole("button", { name: "Next" });
      const amountInput = getByRole("spinbutton");

      await fireEvent.click(useMaxButton);

      expect(amountInput).toHaveValue(maxSpendableDusk);
      expect(nextButton).toBeEnabled();
    });

    it("should not change the default amount (1) in the textbox if the user clicks the related button and the balance is zero", async () => {
      const props = {
        ...baseProps,
        availableBalance: 0n,
      };
      const { getByRole } = render(Send, props);

      await fireEvent.click(getByRole("button", { name: "Next" }));

      const useMaxButton = getByRole("button", { name: "USE MAX" });
      const amountInput = getByRole("spinbutton");

      expect(amountInput).toHaveValue(1);

      await fireEvent.click(useMaxButton);

      expect(amountInput).toHaveValue(1);
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

      const { getByRole } = render(Send, props);

      await fireEvent.click(getByRole("button", { name: "Next" }));

      const useMaxButton = getByRole("button", { name: "USE MAX" });
      const amountInput = getByRole("spinbutton");

      expect(amountInput).toHaveValue(1);

      await fireEvent.click(useMaxButton);

      expect(amountInput).toHaveValue(1);
    });

    it("should disable the next button if the user enters an invalid amount", async () => {
      const { getByRole } = render(Send, baseProps);

      await fireEvent.click(getByRole("button", { name: "Next" }));

      const nextButton = getByRole("button", { name: "Next" });
      const amountInput = getByRole("spinbutton");

      expect(nextButton).toBeEnabled();

      await fireEvent.input(amountInput, { target: { value: 0 } });

      expect(amountInput).toHaveValue(0);
      expect(nextButton).toBeDisabled();
    });

    it("should require a valid EVM address in memo if sending to BEP20 bridge", async () => {
      const { getByRole, findByText, queryByText, container } = render(Send, {
        ...baseProps,
        availableBalance: 1_000_000_000_000_000n,
      });
      const addressInput = getByRole("textbox");
      const nextButtonAddressStep = getByRole("button", { name: "Next" });

      await fireEvent.input(addressInput, {
        target: { value: bep20BridgeAddress },
      });
      await fireEvent.click(nextButtonAddressStep);

      const nextButtonAmountStep = getByRole("button", { name: "Next" });
      const amountInput = getByRole("spinbutton");
      const memoInput = getAsHTMLElement(container, ".operation__send-memo");

      expect(memoInput).toBeInTheDocument();

      // Banner for required memo should be visible
      expect(
        await findByText("Memo required for bridge operation")
      ).toBeInTheDocument();
      expect(nextButtonAmountStep).toBeDisabled();

      // Enter an invalid EVM address as memo
      await fireEvent.input(memoInput, { target: { value: "invalid-evm" } });
      expect(memoInput).toHaveValue("invalid-evm");

      expect(
        await findByText("Invalid EVM address format")
      ).toBeInTheDocument();
      expect(nextButtonAmountStep).toBeDisabled();

      // Enter a valid EVM address as memo
      const validEvmAddress = "0x9876543210987654321098765432109876543210";

      await fireEvent.input(memoInput, {
        target: { value: validEvmAddress },
      });
      expect(memoInput).toHaveValue(validEvmAddress);

      // Error banners should disappear
      expect(
        queryByText("Memo required for bridge operation")
      ).not.toBeInTheDocument();
      expect(queryByText("Invalid EVM address format")).not.toBeInTheDocument();

      await fireEvent.input(amountInput, { target: { value: "10" } });

      expect(nextButtonAmountStep).toBeEnabled();
    });

    it("should make memo optional and not require EVM format if not sending to BEP20 bridge", async () => {
      const { getByRole, queryByText, container } = render(Send, {
        ...baseProps,
        availableBalance: 1_000_000_000_000_000n,
      });
      const addressInput = getByRole("textbox");
      const nextButtonAddressStep = getByRole("button", { name: "Next" });

      await fireEvent.input(addressInput, {
        target: { value: shieldedAddress },
      });
      await fireEvent.click(nextButtonAddressStep);

      const nextButtonAmountStep = getByRole("button", { name: "Next" });
      const amountInput = getByRole("spinbutton");
      const memoSwitch = getByRole("switch");

      expect(memoSwitch).toBeInTheDocument();
      expect(
        queryByText("Memo required for bridge operation")
      ).not.toBeInTheDocument();
      expect(queryByText("Invalid EVM address format")).not.toBeInTheDocument();

      await fireEvent.input(amountInput, { target: { value: "10" } });
      expect(nextButtonAmountStep).toBeEnabled();

      // Show memo field
      await fireEvent.click(memoSwitch);

      const memoInput = getAsHTMLElement(container, ".operation__send-memo");

      expect(memoInput).toBeInTheDocument();

      await fireEvent.input(memoInput, {
        target: { value: "not-an-evm-address" },
      });
      expect(memoInput).toHaveValue("not-an-evm-address");

      expect(queryByText("Invalid EVM address format")).not.toBeInTheDocument();
      expect(nextButtonAmountStep).toBeEnabled();
    });
  });

  describe("Review step", () => {
    it("should render the Send component Review step", async () => {
      const amount = 2345;
      const { container, getByRole } = render(Send, baseProps);
      const addressInput = getByRole("textbox");

      await fireEvent.input(addressInput, {
        target: { value: shieldedAddress },
      });
      await fireEvent.click(getByRole("button", { name: "Next" }));
      await fireEvent.click(getByRole("switch"));

      const amountInput = getByRole("spinbutton");
      const memoInput = getByRole("textbox");

      await fireEvent.input(amountInput, { target: { value: amount } });
      await fireEvent.input(memoInput, {
        target: { value: "abc-example-memo" },
      });

      await fireEvent.click(getByRole("button", { name: "Next" }));

      const value = getAsHTMLElement(
        container,
        ".operation__review-amount span"
      );
      const key = getAsHTMLElement(
        container,
        ".operation__review-address span"
      );
      const memo = getAsHTMLElement(container, ".operation__review-memo span");

      expect(value.textContent).toBe(baseProps.formatter(amount));
      expect(key.textContent).toBe(shieldedAddress);
      expect(memo.textContent).toBe("abc-example-memo");
      expect(container.firstChild).toMatchSnapshot();
    });
  });

  describe("Send operation", () => {
    vi.useFakeTimers();

    const amount = 567;
    const expectedExplorerLink = `/explorer/transactions/transaction?id=${lastTxId}`;

    afterAll(() => {
      vi.useRealTimers();
    });

    it("should perform a transfer for the desired amount, give a success message and supply a link to see the transaction in the explorer", async () => {
      const { getByRole, getByText } = render(Send, baseProps);
      const addressInput = getByRole("textbox");

      await fireEvent.input(addressInput, {
        target: { value: shieldedAddress },
      });
      await fireEvent.click(getByRole("button", { name: "Next" }));

      const amountInput = getByRole("spinbutton");

      await fireEvent.input(amountInput, { target: { value: amount } });
      await fireEvent.click(getByRole("button", { name: "Next" }));
      await fireEvent.click(getByRole("button", { name: "SEND" }));

      await vi.advanceTimersToNextTimerAsync();

      expect(baseProps.execute).toHaveBeenCalledTimes(1);
      expect(baseProps.execute).toHaveBeenCalledWith(
        shieldedAddress,
        duskToLux(amount),
        "",
        baseProps.gasSettings.gasPrice,
        baseProps.gasSettings.gasLimit
      );

      const explorerLink = getByRole("link", { name: /explorer/i });

      expect(getByText("Transaction created")).toBeInTheDocument();
      expect(explorerLink).toHaveAttribute("target", "_blank");
      expect(explorerLink).toHaveAttribute("href", expectedExplorerLink);
    });

    it("should perform a transfer for the desired amount, with a memo, give a success message and supply a link to see the transaction in the explorer", async () => {
      const { getByRole, getByText } = render(Send, baseProps);
      const addressInput = getByRole("textbox");

      await fireEvent.input(addressInput, {
        target: { value: shieldedAddress },
      });
      await fireEvent.click(getByRole("button", { name: "Next" }));
      await fireEvent.click(getByRole("switch"));

      const amountInput = getByRole("spinbutton");
      const memoInput = getByRole("textbox");

      await fireEvent.input(amountInput, { target: { value: amount } });
      await fireEvent.input(memoInput, {
        target: { value: "abc-example-memo" },
      });

      await fireEvent.click(getByRole("button", { name: "Next" }));
      await fireEvent.click(getByRole("button", { name: "SEND" }));

      await vi.advanceTimersToNextTimerAsync();

      expect(baseProps.execute).toHaveBeenCalledTimes(1);
      expect(baseProps.execute).toHaveBeenCalledWith(
        shieldedAddress,
        duskToLux(amount),
        "abc-example-memo",
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

      const { getByRole, getByText } = render(Send, baseProps);
      const addressInput = getByRole("textbox");

      await fireEvent.input(addressInput, {
        target: { value: shieldedAddress },
      });
      await fireEvent.click(getByRole("button", { name: "Next" }));

      const amountInput = getByRole("spinbutton");

      await fireEvent.input(amountInput, { target: { value: amount } });
      await fireEvent.click(getByRole("button", { name: "Next" }));
      await fireEvent.click(getByRole("button", { name: "SEND" }));
      await vi.advanceTimersToNextTimerAsync();

      expect(baseProps.execute).toHaveBeenCalledTimes(1);
      expect(baseProps.execute).toHaveBeenCalledWith(
        shieldedAddress,
        duskToLux(amount),
        "",
        baseProps.gasSettings.gasPrice,
        baseProps.gasSettings.gasLimit
      );
      expect(getByText("Transaction failed")).toBeInTheDocument();
      expect(getByText(errorMessage)).toBeInTheDocument();
    });

    it("should show the success message, but no explorer link, if the execution promise doesn't resolve with an hash", async () => {
      baseProps.execute.mockResolvedValueOnce(void 0);

      const { getByRole, getByText } = render(Send, baseProps);
      const addressInput = getByRole("textbox");

      await fireEvent.input(addressInput, {
        target: { value: shieldedAddress },
      });
      await fireEvent.click(getByRole("button", { name: "Next" }));

      const amountInput = getByRole("spinbutton");

      await fireEvent.input(amountInput, { target: { value: amount } });
      await fireEvent.click(getByRole("button", { name: "Next" }));
      await fireEvent.click(getByRole("button", { name: "SEND" }));
      await vi.advanceTimersToNextTimerAsync();

      expect(baseProps.execute).toHaveBeenCalledTimes(1);
      expect(baseProps.execute).toHaveBeenCalledWith(
        shieldedAddress,
        duskToLux(amount),
        "",
        baseProps.gasSettings.gasPrice,
        baseProps.gasSettings.gasLimit
      );
      expect(getByText("Transaction created")).toBeInTheDocument();
      expect(() => getByRole("link", { name: /explorer/i })).toThrow();
    });
  });
});
