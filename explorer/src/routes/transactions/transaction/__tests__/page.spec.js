import { describe, expect, it } from "vitest";
import { render } from "@testing-library/svelte";
import Transaction from "../+page.svelte";

describe("Transaction", () => {
  it("should render the Transaction page", () => {
    const { container } = render(Transaction, {});

    expect(container.firstChild).toMatchSnapshot();
  });
});
