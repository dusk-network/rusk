import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { get, readable } from "svelte/store";

import * as appStores from "$app/stores";

import ErrorPage from "../+error.svelte";

describe("Error page", () => {
  const storeWithoutError = {
    ...get(appStores.page),
    error: null,
    status: 200,
  };
  const storeWithError = {
    ...storeWithoutError,
    error: new Error("some error message"),
    status: 500,
  };
  const baseOptions = { props: {}, target: document.body };

  afterEach(cleanup);

  it("should render the error page", async () => {
    const pageStoreSpy = vi
      .spyOn(appStores, "page", "get")
      .mockReturnValue(readable(storeWithError));
    const { container } = render(ErrorPage, baseOptions);

    expect(container).toMatchSnapshot();
    pageStoreSpy.mockRestore();
  });

  it("should be able to render the error page even without an error", () => {
    const pageStoreSpy = vi
      .spyOn(appStores, "page", "get")
      .mockReturnValue(readable(storeWithoutError));
    const { container } = render(ErrorPage, baseOptions);

    expect(container).toMatchSnapshot();
    pageStoreSpy.mockRestore();
  });
});
