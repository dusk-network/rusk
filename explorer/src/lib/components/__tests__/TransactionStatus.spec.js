import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";

import { TransactionStatus } from "..";

describe("TransactionStatus", () => {
  const baseProps = {
    errorMessage: "",
  };
  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(cleanup);

  it("should render the transaction status", () => {
    const { container } = render(TransactionStatus, baseOptions);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should pass additional class names to the rendered `Badge` component", () => {
    const props = {
      ...baseProps,
      className: "foo bar",
    };

    const { container } = render(TransactionStatus, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should render the status as error if it receives an error message", () => {
    const props = {
      ...baseProps,
      errorMessage: "Transaction failed",
    };

    const { container } = render(TransactionStatus, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should add tooltip info if it receives an error message and the related property is set", () => {
    const props = {
      ...baseProps,
      errorMessage: "Transaction failed",
      showErrorTooltip: true,
    };

    const { container } = render(TransactionStatus, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("shouldn't add tooltip info if there's no error message, even if the related property is set to `true`", () => {
    const props = {
      ...baseProps,
      showErrorTooltip: true,
    };

    const { container } = render(TransactionStatus, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });
});
