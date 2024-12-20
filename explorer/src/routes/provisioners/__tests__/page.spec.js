import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { get } from "svelte/store";

import { duskAPI } from "$lib/services";
import { appStore } from "$lib/stores";
import { hostProvisioners } from "$lib/mock-data";
import { changeMediaQueryMatches } from "$lib/dusk/test-helpers";

import Provisioners from "../+page.svelte";

describe("Provisioners page", () => {
  vi.useFakeTimers();

  const { provisionersFetchInterval } = get(appStore);
  const getProvisionersSpy = vi
    .spyOn(duskAPI, "getProvisioners")
    .mockResolvedValue(hostProvisioners);

  afterEach(async () => {
    await vi.runOnlyPendingTimersAsync();

    cleanup();
    getProvisionersSpy.mockClear();
  });

  afterAll(() => {
    vi.useRealTimers();
    getProvisionersSpy.mockRestore();
  });

  it("should render the Provisioners page, start polling for provisioners and stop the polling when unmounted", async () => {
    const { container, unmount } = render(Provisioners);

    // snapshost in loading state
    expect(container.firstChild).toMatchSnapshot();
    expect(getProvisionersSpy).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(1);

    // snapshot with received data from GraphQL
    expect(container.firstChild).toMatchSnapshot();

    await vi.advanceTimersByTimeAsync(provisionersFetchInterval - 1);

    expect(getProvisionersSpy).toHaveBeenCalledTimes(2);

    await vi.advanceTimersByTimeAsync(provisionersFetchInterval);

    expect(getProvisionersSpy).toHaveBeenCalledTimes(3);

    unmount();

    await vi.advanceTimersByTimeAsync(provisionersFetchInterval * 10);

    expect(getProvisionersSpy).toHaveBeenCalledTimes(3);
  });

  it("should render the Provisioners page with the mobile layout", async () => {
    const { container } = render(Provisioners);

    changeMediaQueryMatches("(max-width: 1024px)", true);

    expect(get(appStore).isSmallScreen).toBe(true);

    expect(getProvisionersSpy).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(1);

    expect(container.firstChild).toMatchSnapshot();
  });
});
