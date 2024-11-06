import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { skipIn } from "lamb";
import { Balance } from "..";

describe("Balance", () => {
  const baseProps = {
    fiatCurrency: "USD",
    fiatPrice: 10,
    locale: "en",
    shieldedAmount: 1_000_000n,
    tokenCurrency: "DUSK",
    unshieldedAmount: 2_000_000n,
  };

  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(cleanup);

  it("renders the Balance component", () => {
    const { container } = render(Balance, baseOptions);

    expect(
      container.querySelector(".dusk-balance__usage-details")
    ).toBeInTheDocument();
    expect(container.firstChild).toMatchSnapshot();
  });

  it("should update the Balance component when the props change", async () => {
    const { container, rerender } = render(Balance, baseOptions);

    expect(container.firstChild).toMatchSnapshot();

    await rerender({
      fiatCurrency: "EUR",
      fiatPrice: 20,
      locale: "it",
      shieldedAmount: 500_000n,
      tokenCurrency: "DUSK",
      unshieldedAmount: 2_500_000n,
    });

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should display the right percentage values", async () => {
    const { container } = render(Balance, baseOptions);

    // Check if shielded percentage displays as 33.33%
    expect(
      container.querySelector(
        ".dusk-balance__account:first-child .dusk-balance__percentage"
      )?.textContent
    ).toContain("33.33%");

    // Check if unshielded percentage displays as 66.67%
    expect(
      container.querySelector(
        ".dusk-balance__account:last-child .dusk-balance__percentage"
      )?.textContent
    ).toContain("66.67%");
  });

  it("should display the right percentage values when balance is zero", async () => {
    const options = {
      ...baseOptions,
      props: { ...baseProps, shieldedAmount: 0n, unshieldedAmount: 0n },
    };

    const { container } = render(Balance, options);

    expect(
      container.querySelector(
        ".dusk-balance__account:first-child .dusk-balance__percentage"
      )?.textContent
    ).toContain("0%");

    expect(
      container.querySelector(
        ".dusk-balance__account:last-child .dusk-balance__percentage"
      )?.textContent
    ).toContain("0%");
  });

  it("should pass additional class names and attributes to the rendered element", async () => {
    const props = {
      ...baseProps,
      className: "foo bar",
      id: "balance",
    };
    const { container, rerender } = render(Balance, { ...baseOptions, props });

    expect(container.firstChild).toHaveClass("foo bar");
    expect(container.firstChild).toHaveAttribute("id", "balance");

    await rerender({
      ...props,
      className: "qux",
      id: "new-balance",
    });

    expect(container.firstChild).toHaveClass("qux");
    expect(container.firstChild).toHaveAttribute("id", "new-balance");
  });

  it("should not display the fiat value if the fiat price is `undefined`", () => {
    const props = skipIn(baseProps, ["fiatPrice"]);
    const { container } = render(Balance, { ...baseOptions, props });

    expect(container.querySelector(".dusk-balance__fiat--visible")).toBeNull();
    expect(container.firstChild).toMatchSnapshot();
  });
});
