import {
	afterEach,
	describe,
	expect,
	it,
	vi
} from "vitest";
import { cleanup, render } from "@testing-library/svelte";

import { OperationResult } from "..";

vi.useFakeTimers();

/** @type {(delay: number) => Promise<any>} */
const rejectAfter = delay => new Promise((_, reject) => {
	setTimeout(() => reject(new Error("some error")), delay);
});

/** @type {(delay: number) => Promise<any>} */
const resolveAfter = delay => new Promise(resolve => { setTimeout(resolve, delay); });

describe("OperationResult", () => {
	const delay = 1000;

	const onBeforeLeave = vi.fn();

	const baseProps = {
		onBeforeLeave,
		operation: resolveAfter(delay)
	};

	const baseOptions = {
		props: baseProps,
		target: document.body
	};

	afterEach(() => {
		cleanup();
		onBeforeLeave.mockClear();
	});

	it("should be able to render the `OperationResult` component in a pending state", () => {
		const { container } = render(OperationResult, baseOptions);

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should accept a custom message for the pending state", () => {
		const props = {
			...baseProps,
			pendingMessage: "Transaction pending"
		};
		const { container } = render(OperationResult, { ...baseOptions, props });

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should be able to render the `OperationResult` in a successful state", async () => {
		const { container } = render(OperationResult, baseOptions);

		await vi.advanceTimersByTimeAsync(delay);

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should accept a custom message for the successful state", async () => {
		const props = {
			...baseProps,
			successMessage: "Transaction completed"
		};

		const { container } = render(OperationResult, { ...baseOptions, props });

		await vi.advanceTimersByTimeAsync(delay);

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should call the `onBeforeLeave` function when the home button is clicked", async () => {
		const { getByRole } = render(OperationResult, baseOptions);

		await vi.advanceTimersByTimeAsync(delay);

		const homeBtn = getByRole("link");

		homeBtn.click();

		expect(baseProps.onBeforeLeave).toHaveBeenCalledTimes(1);
	});

	it("should be able to render the `OperationResult` in a failure state", async () => {
		const props = {
			...baseProps,
			operation: rejectAfter(delay)
		};

		const { container } = render(OperationResult, { ...baseOptions, props });

		await vi.advanceTimersByTimeAsync(delay);

		expect(container.firstChild).toMatchSnapshot();
	});

	it("should accept a custom message for the failure state", async () => {
		const props = {
			...baseProps,
			errorMessage: "Transaction failed",
			operation: rejectAfter(delay)
		};

		const { container } = render(OperationResult, { ...baseOptions, props });

		await vi.advanceTimersByTimeAsync(delay);

		expect(container.firstChild).toMatchSnapshot();
	});
});
