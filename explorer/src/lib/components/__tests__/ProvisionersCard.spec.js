import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { slice } from "lamb";

import { hostProvisioners } from "$lib/mock-data";
import { ProvisionersCard } from "..";

describe("Provisioners Card", () => {
  const data = slice(hostProvisioners, 0, 10);

  const baseProps = {
    error: null,
    isSmallScreen: false,
    loading: false,
    provisioners: null,
  };
  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(cleanup);

  it("should render the `ProvisionersCard` component", () => {
    const { container } = render(ProvisionersCard, baseOptions);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should disable the `Show More` button is the card is in the loading state", () => {
    const loading = true;

    const { container, getByRole } = render(ProvisionersCard, {
      ...baseOptions,
      props: { ...baseProps, loading },
    });

    expect(getByRole("button", { name: "Show More" })).toBeDisabled();

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should disable the `Show More` button if there is no more data to display", async () => {
    const loading = false;
    const provisioners = data;

    const { container, getByRole } = render(ProvisionersCard, {
      ...baseOptions,
      props: { ...baseProps, loading, provisioners },
    });

    expect(getByRole("button", { name: "Show More" })).toBeDisabled();

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should hide the `Show More` button if an error has occurred", async () => {
    const props = { ...baseProps, error: new Error("error") };

    const { container } = render(ProvisionersCard, {
      ...baseOptions,
      props: { ...props, error: new Error("error") },
    });

    expect(container.firstChild).toMatchSnapshot();
  });
});
