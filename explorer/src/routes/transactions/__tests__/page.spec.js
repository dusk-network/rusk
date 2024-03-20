import { describe, expect, it } from "vitest";
import { render } from "@testing-library/svelte";
import Transactions from "../+page.svelte";

describe("Transactions", () => {
  it("should render the Transactions page", () => {
    const { container } = render(Transactions, {});

    expect(container.firstChild).toMatchSnapshot();
  });
});
