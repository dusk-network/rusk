import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";

import mockedWalletStore from "../../../../../__mocks__/mockedWalletStore";
import { createCurrencyFormatter, luxToDusk } from "$lib/dusk/currency";

import SendPage from "../+page.svelte";

vi.mock("$lib/dusk/string", async (importOriginal) => {
  /** @type {typeof import("$lib/dusk/string")} */
  const original = await importOriginal();

  return {
    ...original,
    randomUUID: () => "some-generated-id",
  };
});

vi.mock("$lib/stores", async (importOriginal) => {
  /** @type {typeof import("$lib/stores")} */
  const original = await importOriginal();

  return {
    ...original,
    walletStore: { ...original.walletStore, ...mockedWalletStore },
  };
});

const formatter = createCurrencyFormatter("en", "DUSK", 9);
const { balance, currentProfile } = mockedWalletStore.getMockedStoreValue();
const formattedShielded = formatter(luxToDusk(balance.shielded.spendable));
const formattedUnshielded = formatter(luxToDusk(balance.unshielded.value));

describe("Send page", () => {
  afterEach(cleanup);

  afterAll(() => {
    vi.doUnmock("$lib/dusk/string");
    vi.doUnmock("$lib/stores");
  });

  it("should render the send page", async () => {
    const { container } = render(SendPage);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should update the spendable amount passed to its child components when a `keyChange` event is fired", async () => {
    const { container, getByRole } = render(SendPage);
    const addressTextbox = getByRole("textbox");

    expect(
      container.querySelector(".contract-statuses__value")
    ).toHaveTextContent(formattedShielded);

    await fireEvent.input(addressTextbox, {
      target: { value: currentProfile.account.toString() },
    });

    expect(
      container.querySelector(".contract-statuses__value")
    ).toHaveTextContent(formattedUnshielded);

    await fireEvent.input(addressTextbox, {
      target: { value: currentProfile.address.toString() },
    });

    expect(
      container.querySelector(".contract-statuses__value")
    ).toHaveTextContent(formattedShielded);
  });
});
