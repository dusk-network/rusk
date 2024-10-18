import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render } from "@testing-library/svelte";
import { slice } from "lamb";

import { hostProvisioners } from "$lib/mock-data";

import { ProvisionersList } from "..";

describe("Provisioners List", () => {
  const provisioners = slice(hostProvisioners, 0, 1)[0];

  /** @type {import("svelte").ComponentProps<ProvisionersList>} */
  const baseProps = { data: provisioners };

  afterEach(cleanup);

  it("renders the `ProvisionersList` component", () => {
    const { container } = render(ProvisionersList, baseProps);

    expect(container.firstChild).toMatchSnapshot();
  });
});
