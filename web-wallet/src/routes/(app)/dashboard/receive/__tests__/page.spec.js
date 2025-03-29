import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { get } from "svelte/store";

import mockedWalletStore from "$lib/__mocks__/mockedWalletStore";

import { walletStore } from "$lib/stores";

import Receive from "../+page.svelte";

vi.useFakeTimers();

vi.mock("$lib/dusk/string", async (importOriginal) => {
  /** @type {typeof import("$lib/dusk/string")} */
  const original = await importOriginal();

  return {
    ...original,
    randomUUID: () => "some-generated-id",
  };
});

vi.mock("$lib/stores", async (importOriginal) => {
  /** @type {WalletStore} */
  const original = await importOriginal();

  return {
    ...original,
    walletStore: {
      ...mockedWalletStore,
    },
  };
});

describe("Receive", () => {
  const currentProfile = /** @type {Profile} */ (
    get(walletStore).currentProfile
  );
  const expectedAddress = currentProfile.address.toString();
  const expectedAccount = currentProfile.account.toString();

  afterEach(cleanup);

  afterAll(() => {
    vi.doUnmock("$lib/dusk/string");
    vi.doUnmock("$lib/stores");
    vi.useRealTimers();
  });

  it("should render the receive page with a choice to switch from public to shielded address", async () => {
    const { container, getByRole, getByText } = render(Receive);

    await vi.runAllTimersAsync();

    expect(getByRole("radiogroup")).toBeInTheDocument();
    expect(getByText(expectedAccount)).toBeInTheDocument();
    expect(() => getByText(expectedAddress)).toThrow();
    expect(container.firstChild).toMatchSnapshot();
  });

  it("should show the shielded address when the user makes such choice", async () => {
    const { container, getByRole, getByText } = render(Receive);

    await vi.runAllTimersAsync();

    expect(getByText(expectedAccount)).toBeInTheDocument();
    expect(() => getByText(expectedAddress)).toThrow();

    await fireEvent.click(getByRole("radio", { checked: false }));
    await vi.runAllTimersAsync();

    expect(() => getByText(expectedAccount)).toThrow();
    expect(getByText(expectedAddress)).toBeInTheDocument();
    expect(container.firstChild).toMatchSnapshot();
  });
});
