import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { CopyField } from "..";

Object.assign(navigator, {
  clipboard: {
    writeText: vi.fn().mockResolvedValue(""),
  },
});

describe("CopyField", () => {
  const baseProps = {
    disabled: false,
    displayValue: "1,234,567",
    name: "Sample Information",
    rawValue: "1234567",
  };

  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(cleanup);

  it("renders the CopyField component", () => {
    const { container } = render(CopyField, baseOptions);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("renders the CopyField component with the copy button disabled", () => {
    const { container, getByRole } = render(CopyField, {
      ...baseOptions,
      props: { ...baseProps, disabled: true },
    });

    const copyButton = getByRole("button");

    expect(copyButton).toBeDisabled();

    expect(container.firstChild).toMatchSnapshot();
  });

  it("copies the raw value on pressing the copy button", async () => {
    const { getByRole } = render(CopyField, baseOptions);

    const copyButton = getByRole("button");

    await fireEvent.click(copyButton);

    expect(navigator.clipboard.writeText).toHaveBeenCalledWith("1234567");
  });
});
