import {
  afterAll,
  afterEach,
  beforeAll,
  describe,
  expect,
  it,
  vi,
} from "vitest";
import { cleanup, fireEvent, render } from "@testing-library/svelte";
import { createCurrencyFormatter } from "$lib/dusk/currency";

import { Unstake } from "..";
import { mdiDatabaseArrowDownOutline, mdiGiftOpenOutline } from "@mdi/js";

vi.mock("$lib/dusk/string", async (importOriginal) => {
  /** @type {typeof import("$lib/dusk/string")} */
  const original = await importOriginal();

  return {
    ...original,
    randomUUID: () => "some-generated-id",
  };
});

describe("Unstake", () => {
  const formatter = createCurrencyFormatter("en", "DUSK", 9);
  const lastTxId = "some-id";

  const baseProps = {
    execute: vi.fn().mockResolvedValue(lastTxId),
    formatter,
    gasLimits: {
      gasLimitLower: 10000000n,
      gasLimitUpper: 2900000000n,
      gasPriceLower: 1n,
    },
    gasSettings: {
      gasLimit: 20000000n,
      gasPrice: 1n,
    },
  };

  afterEach(() => {
    cleanup();
    baseProps.execute.mockClear();
  });

  describe("Unstake operation", () => {
    const expectedExplorerLink = `/explorer/transactions/transaction?id=${lastTxId}`;

    const unstakeProps = {
      maxAmount: 278_000_000_000n,
      operationCtaIconPath: mdiDatabaseArrowDownOutline,
      operationCtaLabel: "Unstake",
      operationOverviewLabel: "Unstake Amount",
    };

    beforeAll(() => {
      vi.useFakeTimers();
    });

    afterAll(() => {
      vi.useRealTimers();
    });

    it("should render the unstake view", () => {
      const baseOptions = {
        props: {
          ...baseProps,
          ...unstakeProps,
        },
        target: document.body,
      };

      const { container, getByRole } = render(Unstake, baseOptions);
      const nextButton = getByRole("button", { name: "Unstake" });

      expect(nextButton).toBeEnabled();
      expect(container.firstChild).toMatchSnapshot();
    });

    it("should perform an ustake, give a success message and supply a link to see the transaction in the explorer", async () => {
      /** @type {import("svelte").ComponentProps<Unstake>} */
      const props = { ...baseProps, ...unstakeProps };

      const { getByRole, getByText } = render(Unstake, props);

      await vi.advanceTimersToNextTimerAsync();

      await fireEvent.click(getByRole("button", { name: "Unstake" }));

      expect(baseProps.execute).toHaveBeenCalledTimes(1);
      expect(baseProps.execute).toHaveBeenCalledWith(
        baseProps.gasSettings.gasPrice,
        baseProps.gasSettings.gasLimit
      );

      const explorerLink = getByRole("link", { name: /explorer/i });

      expect(getByText("Transaction created")).toBeInTheDocument();
      expect(explorerLink).toHaveAttribute("target", "_blank");
      expect(explorerLink).toHaveAttribute("href", expectedExplorerLink);
    });

    it("should not allow to unstake, if wrong gas settings are provided", async () => {
      /** @type {import("svelte").ComponentProps<Unstake>} */
      const props = {
        ...baseProps,
        ...unstakeProps,
        gasSettings: { gasLimit: 29000000090n, gasPrice: 1n },
      };

      const { getByRole } = render(Unstake, props);

      await vi.advanceTimersToNextTimerAsync();

      const unstakeButton = getByRole("button", { name: "Unstake" });

      expect(unstakeButton).toBeDisabled();
    });
  });

  describe("Claim Rewards operation", () => {
    const expectedExplorerLink = `/explorer/transactions/transaction?id=${lastTxId}`;

    const claimRewardsProps = {
      maxAmount: 345_000_000_000n,
      operationCtaIconPath: mdiGiftOpenOutline,
      operationCtaLabel: "Claim Rewards",
      operationOverviewLabel: "Rewards Amount",
    };

    beforeAll(() => {
      vi.useFakeTimers();
    });

    afterAll(() => {
      vi.useRealTimers();
    });

    it("should render the claim rewards view", () => {
      const baseOptions = {
        props: {
          ...baseProps,
          ...claimRewardsProps,
        },
        target: document.body,
      };

      const { container, getByRole } = render(Unstake, baseOptions);
      const nextButton = getByRole("button", { name: "Claim Rewards" });

      expect(nextButton).toBeEnabled();
      expect(container.firstChild).toMatchSnapshot();
    });

    it("should perform a claim rewards, give a success message and supply a link to see the transaction in the explorer", async () => {
      /** @type {import("svelte").ComponentProps<Unstake>} */
      const props = { ...baseProps, ...claimRewardsProps };

      const { getByRole, getByText } = render(Unstake, props);

      await vi.advanceTimersToNextTimerAsync();

      await fireEvent.click(getByRole("button", { name: "Claim Rewards" }));

      expect(baseProps.execute).toHaveBeenCalledTimes(1);
      expect(baseProps.execute).toHaveBeenCalledWith(
        baseProps.gasSettings.gasPrice,
        baseProps.gasSettings.gasLimit
      );

      const explorerLink = getByRole("link", { name: /explorer/i });

      expect(getByText("Transaction created")).toBeInTheDocument();
      expect(explorerLink).toHaveAttribute("target", "_blank");
      expect(explorerLink).toHaveAttribute("href", expectedExplorerLink);
    });

    it("should not allow to claim rewards, if wrong gas settings are provided", async () => {
      /** @type {import("svelte").ComponentProps<Unstake>} */
      const props = {
        ...baseProps,
        ...claimRewardsProps,
        gasSettings: { gasLimit: 29000000090n, gasPrice: 1n },
      };

      const { getByRole } = render(Unstake, props);

      await vi.advanceTimersToNextTimerAsync();

      const claimButton = getByRole("button", { name: "Claim Rewards" });

      expect(claimButton).toBeDisabled();
    });
  });
});
