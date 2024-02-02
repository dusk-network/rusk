import {
	afterAll,
	afterEach,
	beforeEach,
	describe,
	expect,
	it,
	vi
} from "vitest";
import { act, cleanup } from "@testing-library/svelte";

import { renderWithSimpleContent } from "$lib/dusk/test-helpers";
import mockedWalletStore from "../../__mocks__/mockedWalletStore";
import Layout from "../+layout.svelte";

vi.mock("$lib/stores", async importOriginal => {
	/** @type {import("$lib/stores/stores").WalletStore} */
	const original = await importOriginal();

	return {
		...original,
		walletStore: mockedWalletStore
	};
});

describe("Dashboard Layout", () => {
	/**
	 * @param {Element} container
	 * @param {"error" | "success" | "warning"} status
	 * @returns
	 */
	const getStatusWrapper = (container, status) =>
		container.querySelector(`.footer__network-status-icon--${status}`);
	const initialState = structuredClone(mockedWalletStore.getMockedStoreValue());

	beforeEach(() => {
		mockedWalletStore.setMockedStoreValue(initialState);
	});

	afterEach(cleanup);

	afterAll(() => {
		vi.doUnmock("$lib/stores");
	});

	it("should render the dashboard layout", () => {
		const { container } = renderWithSimpleContent(Layout, {});

		expect(getStatusWrapper(container, "success")).toBeTruthy();
		expect(container.firstChild).toMatchSnapshot();
	});

	it("should render the dashboard layout in the sync state", async () => {
		const { container } = renderWithSimpleContent(Layout, {});

		expect(getStatusWrapper(container, "warning")).toBeNull();

		await act(() => {
			mockedWalletStore.setMockedStoreValue({
				...initialState,
				isSyncing: true
			});
		});

		expect(getStatusWrapper(container, "warning")).toBeTruthy();
		expect(container.firstChild).toMatchSnapshot();

		await act(() => {
			mockedWalletStore.setMockedStoreValue({ initialState });
		});

		expect(getStatusWrapper(container, "warning")).toBeNull();
	});

	it("should render the dashboard layout in the error state", async () => {
		const { container } = renderWithSimpleContent(Layout, {});
		const getRetryButton = () => container.querySelector(".footer__actions-button");

		expect(getStatusWrapper(container, "error")).toBeNull();
		expect(getRetryButton()).toBeNull();

		await act(() => {
			mockedWalletStore.setMockedStoreValue({
				...initialState,
				error: new Error()
			});
		});

		expect(getStatusWrapper(container, "error")).toBeTruthy();
		expect(getRetryButton()).toBeTruthy();
		expect(container.firstChild).toMatchSnapshot();

		await act(() => {
			mockedWalletStore.setMockedStoreValue({ initialState });
		});

		expect(getStatusWrapper(container, "error")).toBeNull();
		expect(getRetryButton()).toBeNull();
		expect(getStatusWrapper(container, "success")).toBeTruthy();
	});
});
