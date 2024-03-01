import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { QrCode } from "..";

describe("QrCode", () => {
  const baseProps = {
    value: "xyz",
  };
  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(cleanup);

  it("should render the QrCode component", () => {
    const { container } = render(QrCode, baseOptions);

    expect(container.firstChild).toMatchSnapshot();
  });
});
