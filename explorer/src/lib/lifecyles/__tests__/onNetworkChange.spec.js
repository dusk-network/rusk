import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import * as Svelte from "svelte";
import { get } from "svelte/store";

import { appStore } from "$lib/stores";

import { onNetworkChange } from "..";

describe("onNetworkChange", () => {
  /** @type {Function} */
  let unsubscriber;

  const onDestroySpy = vi
    .spyOn(Svelte, "onDestroy")
    .mockImplementation((fn) => {
      unsubscriber = fn;
    });

  afterEach(() => {
    onDestroySpy.mockClear();
  });

  afterAll(() => {
    onDestroySpy.mockRestore();
  });

  it("should create a custom lifecycle that calls the given callback when the network changes in the `appStore`", () => {
    const { network } = get(appStore);
    const callback = vi.fn();

    onNetworkChange(callback);

    expect(callback).toHaveBeenCalledTimes(1);
    expect(callback).toHaveBeenCalledWith(network);

    appStore.setNetwork("some-network");

    expect(callback).toHaveBeenCalledTimes(2);
    expect(callback).toHaveBeenNthCalledWith(2, "some-network");

    appStore.setNetwork("some-network");

    expect(callback).toHaveBeenCalledTimes(2);

    unsubscriber();

    appStore.setNetwork("some-other-network");

    expect(callback).toHaveBeenCalledTimes(2);
  });
});
