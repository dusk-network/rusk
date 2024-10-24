import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { get } from "svelte/store";

import mockedWalletStore from "../../../__mocks__/mockedWalletStore";

import { AddressPicker } from "..";

describe("AddressPicker", () => {
  const { currentProfile, profiles } = get(mockedWalletStore);

  const props = { currentProfile, profiles };

  beforeEach(() => {
    Object.assign(navigator, {
      clipboard: {
        writeText: vi.fn().mockResolvedValue(undefined),
      },
    });
  });

  afterEach(cleanup);

  it("renders the AddressPicker component", () => {
    const { container } = render(AddressPicker, props);

    expect(container.firstElementChild).toMatchSnapshot();
  });

  it("should be able to render the component if the current profile is `null`", () => {
    const { container } = render(AddressPicker, {
      ...props,
      currentProfile: null,
    });

    expect(container.firstElementChild).toMatchSnapshot();
  });

  it("copies the current address on Copy button click", async () => {
    const { getByRole } = render(AddressPicker, props);

    const component = getByRole("button", { name: "Copy Address" });

    await fireEvent.click(component);

    expect(navigator.clipboard.writeText).toHaveBeenCalledWith(
      currentProfile.address.toString()
    );
  });
});
