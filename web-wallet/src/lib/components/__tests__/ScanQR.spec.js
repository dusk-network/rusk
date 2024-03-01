import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";

import { ScanQR } from "..";

describe("ScanQR", () => {
  const baseProps = {
    scanner: undefined,
  };

  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(cleanup);

  it("renders the ScanQR component", () => {
    const { container } = render(ScanQR, baseOptions);

    expect(container.firstChild).toMatchSnapshot();
  });
});
