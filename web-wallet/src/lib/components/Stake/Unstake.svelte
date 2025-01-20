<svelte:options immutable={true} />

<script>
  import { createEventDispatcher, onMount } from "svelte";
  import { fade } from "svelte/transition";

  import { MESSAGES } from "$lib/constants";
  import { areValidGasSettings } from "$lib/contracts";
  import { duskToLux, luxToDusk } from "$lib/dusk/currency";

  import {
    AnchorButton,
    Badge,
    Button,
    Icon,
    Stepper,
    Textbox,
    Wizard,
    WizardStep,
  } from "$lib/dusk/components";
  import {
    Banner,
    GasFee,
    GasSettings,
    OperationResult,
  } from "$lib/components";

  import StakeOverview from "./StakeOverview.svelte";
  import { logo } from "$lib/dusk/icons";
  import { toast } from "$lib/dusk/components/Toast/store";
  import { mdiAlertOutline } from "@mdi/js";

  /** @type {(...args: any[]) => Promise<string>} */
  export let execute;

  /** @type {(amount: number) => string} */
  export let formatter;

  /** @type {GasStoreContent} */
  export let gasLimits;

  /** @type {ContractGasSettings} */
  export let gasSettings;

  /** @type {bigint} */
  export let maxWithdrawAmount;

  /** @type {string} */
  export let operationCtaLabel;

  /** @type {string} */
  export let operationCtaIconPath;

  /** @type {string} */
  export let operationOverviewLabel;

  /** @type {bigint} */
  export let availableBalance;

  /**
   * @type {bigint|undefined}
   * If `minStakeRequirement` is undefined, it indicates that the user is claiming rewards.
   * If it is defined, it means the user is performing an unstaking operation.
   */
  export let minStakeRequirement;

  let activeStep = 0;
  let { gasLimit, gasPrice } = gasSettings;
  let isGasValid = false;

  /**
   * We are forced to keep `amount`
   * as number if we want to use
   * Svelte's binding.
   */
  let withdrawAmount = luxToDusk(maxWithdrawAmount);

  const steps = [{ label: "Amount" }, { label: "Overview" }, { label: "Done" }];

  const dispatch = createEventDispatcher();
  const resetOperation = () => dispatch("operationChange", "");

  /**
   * @param {{detail: {price: bigint, limit: bigint}}} event
   */
  const setGasValues = (event) => {
    isGasValid = areValidGasSettings(event.detail.price, event.detail.limit);

    if (isGasValid) {
      gasPrice = event.detail.price;
      gasLimit = event.detail.limit;
    }
  };

  function setMaxAmount() {
    if (!isGasValid) {
      toast("error", "Please set valid gas settings first", mdiAlertOutline);
      return;
    }

    if (availableBalance < maxGasFee) {
      toast(
        "error",
        "You don't have enough DUSK to cover the transaction fee",
        mdiAlertOutline
      );
      return;
    }

    withdrawAmount = luxToDusk(maxWithdrawAmount);
  }

  onMount(() => {
    isGasValid = areValidGasSettings(gasPrice, gasLimit);
  });

  $: withdrawAmountInLux = withdrawAmount ? duskToLux(withdrawAmount) : 0n;

  // Calculate the maximum gas fee based on the gas limit and gas price.
  $: maxGasFee = gasLimit * gasPrice;

  // Check if the available balance is sufficient to cover the max gas fee.
  // This is a prerequisite for any transaction.
  $: isBalanceSufficientForGas = availableBalance >= maxGasFee;

  // Validate that the unstake amount is within allowable limits:
  // - If unstaking, the remaining amount after unstaking must be at least the minimum stake requirement.
  // - If unstaking everything, the above condition is not applicable.
  // - At most the maximum amount available to withdraw
  $: isWithdrawAmountValid =
    minStakeRequirement !== undefined
      ? (withdrawAmountInLux === maxWithdrawAmount ||
          maxWithdrawAmount - withdrawAmountInLux >= minStakeRequirement) &&
        withdrawAmountInLux <= maxWithdrawAmount
      : withdrawAmountInLux <= maxWithdrawAmount;

  $: isNextButtonDisabled = !(
    isWithdrawAmountValid &&
    isGasValid &&
    isBalanceSufficientForGas
  );
</script>

<div class="operation">
  <Wizard steps={3} let:key>
    <div slot="stepper">
      <Stepper {activeStep} {steps} />
    </div>

    <WizardStep
      step={0}
      {key}
      backButton={{
        action: resetOperation,
        disabled: false,
      }}
      nextButton={{
        action: () => activeStep++,
        disabled: isNextButtonDisabled,
      }}
    >
      <div class="operation__amount-wrapper">
        <p>Amount:</p>
        <Button
          size="small"
          variant="tertiary"
          on:click={setMaxAmount}
          text="USE MAX"
        />
      </div>

      <div class="operation__input-wrapper">
        <Textbox
          className="operation__input-field"
          bind:value={withdrawAmount}
          type="number"
          min={minStakeRequirement !== undefined
            ? maxWithdrawAmount - withdrawAmountInLux >= minStakeRequirement
            : undefined}
          max={luxToDusk(maxWithdrawAmount)}
          required
          step="0.000000001"
        />
        <Icon
          data-tooltip-id="main-tooltip"
          data-tooltip-text="DUSK"
          path={logo}
        />
      </div>

      <GasSettings
        {formatter}
        fee={maxGasFee}
        limit={gasSettings.gasLimit}
        limitLower={gasLimits.gasLimitLower}
        limitUpper={gasLimits.gasLimitUpper}
        price={gasSettings.gasPrice}
        priceLower={gasLimits.gasPriceLower}
        on:gasSettings={setGasValues}
      />

      {#if !isBalanceSufficientForGas}
        <Banner variant="error" title="Insufficient balance for gas fees">
          <p>
            Your current balance is too low to cover the required gas fees for
            this transaction. Please deposit additional funds or reduce the gas
            limit.
          </p>
        </Banner>
      {:else if withdrawAmountInLux > maxWithdrawAmount}
        <Banner
          variant="error"
          title="Withdraw amount exceeds available balance"
        >
          <p>
            The amount you are trying to withdraw exceeds the available balance.
            Please reduce the withdraw amount.
          </p>
        </Banner>
      {:else if minStakeRequirement && maxWithdrawAmount !== withdrawAmountInLux && maxWithdrawAmount - withdrawAmountInLux < minStakeRequirement}
        <Banner
          variant="error"
          title="Remaining staked amount below minimum requirement"
        >
          <p>
            The amount you are trying to unstake would leave your remaining
            stake below the minimum staking requirement of {luxToDusk(
              minStakeRequirement
            ).toLocaleString()} DUSK. To proceed, either adjust your unstake amount
            to ensure the remaining stake meets this minimum or click "Use Max" to
            unstake your entire balance.
          </p>
        </Banner>
      {/if}
    </WizardStep>

    <!-- OPERATION OVERVIEW STEP  -->
    <WizardStep
      step={1}
      {key}
      backButton={{
        action: () => activeStep--,
        disabled: false,
      }}
      nextButton={{
        action: () => activeStep++,
        icon: {
          path: operationCtaIconPath,
          position: "before",
        },
        label: operationCtaLabel,
        variant: "primary",
      }}
    >
      <div in:fade|global class="operation__unstake">
        <Badge text="REVIEW TRANSACTION" variant="warning" />
        <StakeOverview
          label={operationOverviewLabel}
          value={formatter(withdrawAmount)}
        />
        <GasFee {formatter} fee={maxGasFee} />
      </div>
    </WizardStep>

    <!-- OPERATION RESULT STEP  -->
    <WizardStep step={2} {key} showNavigation={false}>
      <OperationResult
        errorMessage="Transaction failed"
        onBeforeLeave={resetOperation}
        operation={execute(withdrawAmountInLux, gasPrice, gasLimit)}
        pendingMessage="Processing transaction"
        successMessage="Transaction created"
      >
        <svelte:fragment slot="success-content" let:result={hash}>
          <p>{MESSAGES.TRANSACTION_CREATED}</p>
          {#if hash}
            <AnchorButton
              href={`/explorer/transactions/transaction?id=${hash}`}
              on:click={resetOperation}
              text="VIEW ON BLOCK EXPLORER"
              rel="noopener noreferrer"
              target="_blank"
            />
          {/if}
        </svelte:fragment>
      </OperationResult>
    </WizardStep>
  </Wizard>
</div>

<style lang="postcss">
  .operation {
    &__unstake {
      display: flex;
      flex-direction: column;
      gap: 1.2em;
    }

    &__amount-wrapper,
    &__input-wrapper {
      display: flex;
      align-items: center;
      width: 100%;
    }

    &__amount-wrapper {
      justify-content: space-between;
    }

    &__input-wrapper {
      column-gap: var(--default-gap);
    }
  }
</style>
