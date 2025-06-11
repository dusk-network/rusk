import { afterAll, afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { setPathIn } from "lamb";

import { gqlBlock } from "$lib/mock-data";
import { transformBlock } from "$lib/chain-info";

import { BlockDetails } from "../";

/**
 * @param {HTMLElement} container
 * @param {"next" | "prev"} which
 * @returns {HTMLAnchorElement?}
 */
const getBlockNavLink = (container, which) =>
  container.querySelector(
    `.block-details__list-anchor:nth-of-type(${which === "prev" ? "1" : "2"})`
  );

describe("Block Details", () => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date(2024, 4, 20));

  const baseProps = {
    data: transformBlock(gqlBlock.block),
    error: null,
    loading: false,
    payload:
      '{"version":1,"height":21455,"timestamp":1726246600,"prev_block_hash":"940d0a9a35f30103554e9d74ae504d4a3f679bda7b925df9bff1002424a5a748","seed":"a6b11ca51a17bbe4899e3a924d2c7e087d2836800f276db9cf3e36516b4bf9a99aa8507567cbc3ab9a2c3f0390a47748","state_hash":"70a544fbea9914958dc3dba1824956a1b6de37fbfbc42d2c3e219793b8ce8017","event_hash":"02aee0f39c5936122a5ce52aacf4cf29949dd6d3eec38979ddc1bbb0b6192e34","generator_bls_pubkey":"244Sywxj7PuMHpcPxemaXLcrY5rPgztra6H9Vz8cU1Ro5v23SxKTfVqr2yS7NXAXE1iq59ndn4aMZmYxuzu3Te3e9fokQKTUkYvFxYg2P2E8EEg1gWUbs3AFL2aNx62HQd7r","txroot":"522addb8dae45b71ef281a76c86c6babef0678c8bc2d58d617dc33fa7722b021","faultroot":"0000000000000000000000000000000000000000000000000000000000000000","gas_limit":5000000000,"iteration":0,"prev_block_cert":{"result":{"Valid":"940d0a9a35f30103554e9d74ae504d4a3f679bda7b925df9bff1002424a5a748"},"validation":{"bitset":256882,"aggregate_signature":"877eedaf65d9fab0b309f5f8b01df38a379612854aa6fb63009b2b3d6c92de494d79d8c612daae50e33cd3b1418a9a9e"},"ratification":{"bitset":386423,"aggregate_signature":"a98a84b6ba9a88fde1a063538807aa35b2af1c52360bf564d9c19abd3d22ba9be9f87ce38c97b580fc05df8c920b9da4"}},"failed_iterations":[],"hash":"22c7e588560a133d1c142c2fe4c067e8b500ced43d5f9ae19cf55f1caf4dd899"}',
  };

  afterEach(cleanup);

  afterAll(() => {
    vi.useRealTimers();
  });

  it("renders the Block Details component", () => {
    const { container } = render(BlockDetails, baseProps);

    expect(container.firstChild).toMatchSnapshot();
  });

  it("should disable the previous block link if the prev block hash is empty or if the current height is `0`", async () => {
    const { container, rerender } = render(
      BlockDetails,
      setPathIn(baseProps, "data.header.prevblockhash", "")
    );

    expect(getBlockNavLink(container, "prev")).toHaveAttribute(
      "aria-disabled",
      "true"
    );

    await rerender(baseProps);

    expect(getBlockNavLink(container, "prev")).toHaveAttribute(
      "aria-disabled",
      "false"
    );

    await rerender(setPathIn(baseProps, "data.header.height", 0));

    expect(getBlockNavLink(container, "prev")).toHaveAttribute(
      "aria-disabled",
      "true"
    );
  });

  it("should disable the next block link if the next block hash is empty", () => {
    const { container } = render(
      BlockDetails,
      setPathIn(baseProps, "data.header.nextblockhash", "")
    );

    expect(getBlockNavLink(container, "next")).toHaveAttribute(
      "aria-disabled",
      "true"
    );
  });

  it("should render the Block Details component with the payload visible", async () => {
    const { container, getByRole } = render(BlockDetails, baseProps);

    await fireEvent.click(getByRole("switch"));

    expect(container.firstChild).toMatchSnapshot();
  });
});
