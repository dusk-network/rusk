import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { slice } from "lamb";
import { hostProvisioners } from "$lib/mock-data";

import { ProvisionersTable } from "..";

describe("Provisioners Table", () => {
  const data = slice(hostProvisioners, 0, 10);

  const baseProps = {
    data: data,
  };
  const baseOptions = {
    props: baseProps,
    target: document.body,
  };

  afterEach(cleanup);

  it("should render the `ProvisionersTable` component", () => {
    const { container } = render(ProvisionersTable, baseOptions);

    expect(container.firstChild).toMatchSnapshot();
  });
});
