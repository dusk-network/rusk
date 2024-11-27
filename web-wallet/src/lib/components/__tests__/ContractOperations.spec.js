import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";

import { ContractOperations } from "..";

describe("ContractOperations", () => {
  const baseProps = {
    items: [
      {
        disabled: false,
        id: "send",
        label: "Send",
        primary: true,
      },
      {
        disabled: false,
        id: "receive",
        label: "Receive",
        primary: false,
      },
      {
        disabled: true,
        id: "stake",
        label: "Stake",
        primary: true,
      },
      {
        disabled: false,
        id: "withdraw-stake",
        label: "Withdraw Stake",
        primary: false,
      },
      {
        disabled: false,
        id: "claim-rewards",
        label: "Claim Rewards",
        primary: false,
      },
    ],
  };
  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(cleanup);

  it("should render the `ContractOperations` component", () => {
    const { container } = render(ContractOperations, baseOptions);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should be able to render the component without items", () => {
    const props = {
      ...baseProps,
      items: [],
    };
    const { container } = render(ContractOperations, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should use a default icon if the operation is not on the known list", () => {
    const props = {
      ...baseProps,
      items: [
        {
          disabled: false,
          id: "foo-operation",
          label: "Foo operation",
          primary: true,
        },
      ],
    };
    const { container } = render(ContractOperations, { ...baseOptions, props });

    expect(container.firstChild).toMatchSnapshot();
  });

  it('should dispatch a "operationChange" event when a operation button is clicked', () => {
    const handleOperationChange = vi.fn();
    const { component, getByRole } = render(ContractOperations, baseOptions);
    const btnReceive = getByRole("button", { name: "Receive" });

    component.$on("operationChange", handleOperationChange);

    fireEvent.click(btnReceive);

    expect(handleOperationChange).toHaveBeenCalledTimes(1);
    expect(handleOperationChange.mock.lastCall?.[0].detail).toBe("receive");
  });
});
