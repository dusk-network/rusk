import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render } from "@testing-library/svelte";

import { QrCode } from "..";

describe("QrCode", () => {
  vi.useFakeTimers();

  const baseProps = {
    value: "some text",
  };
  const baseOptions = {
    props: baseProps,
    target: document.body,
  };
  const toDataUrlSpy = vi.spyOn(HTMLCanvasElement.prototype, "toDataURL");

  afterEach(() => {
    cleanup();
    toDataUrlSpy.mockClear();
  });

  afterAll(() => {
    toDataUrlSpy.mockRestore();
    vi.useRealTimers();
  });

  it("should render the QrCode component and update it when any of the prop change", async () => {
    const { container, rerender } = render(QrCode, baseOptions);

    await vi.runAllTimersAsync();

    expect(container.firstChild).toMatchSnapshot();
    expect(toDataUrlSpy).toHaveBeenCalledTimes(1);

    await rerender({ value: "some different text" });

    expect(toDataUrlSpy).toHaveBeenCalledTimes(2);

    await rerender({ bgColor: "#000" });

    expect(toDataUrlSpy).toHaveBeenCalledTimes(3);

    await rerender({ qrColor: "#fff" });

    expect(toDataUrlSpy).toHaveBeenCalledTimes(4);

    await rerender({ width: 500 });

    expect(toDataUrlSpy).toHaveBeenCalledTimes(5);
  });
});
