import {
	afterAll,
	afterEach,
	describe,
	expect,
	it,
	vi
} from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";

import { deductLuxFeeFrom } from "$lib/contracts";
import { createCurrencyFormatter } from "$lib/dusk/currency";
import { getAsHTMLElement } from "$lib/dusk/test-helpers";

import { Send } from "..";
import { tick } from "svelte";

describe("Send", () => {
	const formatter = createCurrencyFormatter("en", "DUSK", 9);
	const lastTxId = "some-id";
	const baseProps = {
		execute: vi.fn().mockResolvedValue(lastTxId),
		formatter,
		gasSettings: {
			gasLimit: 20000000,
			gasLimitLower: 10000000,
			gasLimitUpper: 1000000000,
			gasPrice: 1,
			gasPriceLower: 1
		},
		spendable: 1000,
		statuses: [{
			label: "Spendable",
			value: "1,000.000000000"
		}]
	};
	const address =
		"aB5rL7yC2zK9eV3xH1gQ6fP4jD8sM0iU2oX7wG9nZ8lT3hU4jP5mK8nS6qJ3wF4aA9bB2cC5dD8eE7";

	afterEach(() => {
		cleanup();
		baseProps.execute.mockClear();
	});

	describe("Amount step", () => {
		it("should render the Send component Amount step", () => {
			const { container } = render(Send, baseProps);

			expect(container.firstChild).toMatchSnapshot();
		});

		it("should disable the next button if the amount is invalid on mount", async () => {
			const props = {
				...baseProps,
				gasSettings: {
					...baseProps.gasSettings,
					gasLimit: 40000000,
					gasPrice: 40000000
				}
			};
			const { getByRole } = render(Send, props);
			const next = getByRole("button", { name: "Next" });

			await tick();
			expect(next).toBeDisabled();
		});

		it("should set the max amount in the textbox if the user clicks the related button", async () => {
			const maxSpendable = deductLuxFeeFrom(
				baseProps.spendable,
				baseProps.gasSettings.gasPrice * baseProps.gasSettings.gasLimit
			);
			const { getByRole } = render(Send, baseProps);
			const useMaxButton = getByRole("button", { name: "USE MAX" });
			const nextButton = getByRole("button", { name: "Next" });
			const amountInput = getByRole("spinbutton");

			await fireEvent.click(useMaxButton);

			expect(amountInput).toHaveValue(maxSpendable);
			expect(nextButton).toBeEnabled();
		});

		it("should disable the next button if the user enters an invalid amount", async () => {
			const { getByRole } = render(Send, baseProps);
			const nextButton = getByRole("button", { name: "Next" });
			const amountInput = getByRole("spinbutton");

			expect(nextButton).toBeEnabled();

			await fireEvent.input(amountInput, { target: { value: 0 } });

			expect(amountInput).toHaveValue(0);
			expect(nextButton).toBeDisabled();
		});
	});

	describe("Address step", () => {
		it("should render the Send component Address step", async () => {
			const { container, getByRole } = render(Send, baseProps);
			const nextButton = getByRole("button", { name: "Next" });

			await fireEvent.click(nextButton);

			expect(container.firstChild).toMatchSnapshot();
		});

		it("should disable the next button if the address is empty", async () => {
			const { getByRole } = render(Send, baseProps);

			await fireEvent.click(getByRole("button", { name: "Next" }));

			const nextButton = getByRole("button", { name: "Next" });
			const addressInput = getByRole("textbox");

			expect(addressInput).toHaveValue("");
			expect(nextButton).toBeDisabled();
		});

		it("should enable the next button if the user inputs an address", async () => {
			const { getByRole } = render(Send, baseProps);

			await fireEvent.click(getByRole("button", { name: "Next" }));

			const nextButton = getByRole("button", { name: "Next" });
			const addressInput = getByRole("textbox");

			expect(nextButton).toBeDisabled();

			await fireEvent.input(addressInput, { target: { value: address } });

			expect(addressInput).toHaveValue(address);
			expect(nextButton).toBeEnabled();
		});
	});

	describe("Review step", () => {
		it("should render the Send component Review step", async () => {
			const amount = 2345;
			const { container, getByRole } = render(Send, baseProps);
			const amountInput = getByRole("spinbutton");

			await fireEvent.input(amountInput, { target: { value: amount } });
			await fireEvent.click(getByRole("button", { name: "Next" }));

			const addressInput = getByRole("textbox");

			await fireEvent.input(addressInput, { target: { value: address } });
			await fireEvent.click(getByRole("button", { name: "Next" }));

			const value = getAsHTMLElement(container, ".operation__review-amount span");
			const key = getAsHTMLElement(container, ".operation__review-address span");

			expect(value.textContent).toBe(baseProps.formatter(amount));
			expect(key.textContent).toBe(address);
			expect(container.firstChild).toMatchSnapshot();
		});
	});

	describe("Send operation", () => {
		vi.useFakeTimers();

		const amount = 567;
		const expectedExplorerLink = `https://explorer.dusk.network/transactions/transaction?id=${lastTxId}`;

		afterAll(() => {
			vi.useRealTimers();
		});

		it("should perform a transfer for the desired amount, give a success message and supply a link to see the transaction in the explorer", async () => {
			const { getByRole, getByText } = render(Send, baseProps);
			const amountInput = getByRole("spinbutton");

			await fireEvent.input(amountInput, { target: { value: amount } });
			await fireEvent.click(getByRole("button", { name: "Next" }));

			const input = getByRole("textbox");

			await fireEvent.input(input, { target: { value: address } });
			await fireEvent.click(getByRole("button", { name: "Next" }));
			await fireEvent.click(getByRole("button", { name: "SEND" }));

			await vi.advanceTimersToNextTimerAsync();

			expect(baseProps.execute).toHaveBeenCalledTimes(1);
			expect(baseProps.execute)
				.toHaveBeenCalledWith(
					address,
					amount,
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

			const { getByRole, getByText } = render(Send, baseProps);
			const amountInput = getByRole("spinbutton");

			await fireEvent.input(amountInput, { target: { value: amount } });
			await fireEvent.click(getByRole("button", { name: "Next" }));

			const input = getByRole("textbox");

			await fireEvent.input(input, { target: { value: address } });
			await fireEvent.click(getByRole("button", { name: "Next" }));
			await fireEvent.click(getByRole("button", { name: "SEND" }));

			await vi.advanceTimersToNextTimerAsync();

			expect(baseProps.execute).toHaveBeenCalledTimes(1);
			expect(baseProps.execute)
				.toHaveBeenCalledWith(
					address,
					amount,
					baseProps.gasSettings.gasPrice,
					baseProps.gasSettings.gasLimit
				);
			expect(getByText("Transaction failed")).toBeInTheDocument();
			expect(getByText(errorMessage)).toBeInTheDocument();
		});

		it("should show the success message, but no explorer link, if the execution promise doesn't resolve with an hash", async () => {
			baseProps.execute.mockResolvedValueOnce(void 0);

			const { getByRole, getByText } = render(Send, baseProps);
			const amountInput = getByRole("spinbutton");

			await fireEvent.input(amountInput, { target: { value: amount } });
			await fireEvent.click(getByRole("button", { name: "Next" }));

			const input = getByRole("textbox");

			await fireEvent.input(input, { target: { value: address } });
			await fireEvent.click(getByRole("button", { name: "Next" }));
			await fireEvent.click(getByRole("button", { name: "SEND" }));

			await vi.advanceTimersToNextTimerAsync();

			expect(baseProps.execute).toHaveBeenCalledTimes(1);
			expect(baseProps.execute)
				.toHaveBeenCalledWith(
					address,
					amount,
					baseProps.gasSettings.gasPrice,
					baseProps.gasSettings.gasLimit
				);
			expect(getByText("Transaction completed")).toBeInTheDocument();
			expect(() => getByRole("link", { name: /explorer/i })).toThrow();
		});
	});
});
