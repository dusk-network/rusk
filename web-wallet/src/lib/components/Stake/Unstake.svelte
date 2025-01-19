<svelte:options immutable={true} />

<script>
  import { createEventDispatcher, onMount } from "svelte";
  import { fade } from "svelte/transition";

  import { MESSAGES } from "$lib/constants";
  import { areValidGasSettings } from "$lib/contracts";
  import { luxToDusk } from "$lib/dusk/currency";

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
  import { GasSettings, OperationResult } from "$lib/components";

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
  export let maxAmount;

  /** @type {string} */
  export let operationCtaLabel;

  /** @type {string} */
  export let operationCtaIconPath;

  /** @type {string} */
  export let operationOverviewLabel;

  let activeStep = 0;
  let { gasLimit, gasPrice } = gasSettings;
  let isGasValid = false;

  /**
   * We are forced to keep `amount`
   * as number if we want to use
   * Svelte's binding.
   */
  let amount = luxToDusk(maxAmount);

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

    // if (spendable < fee) {
    //   toast(
    //     "error",
    //     "You don't have enough DUSK to cover the transaction fee",
    //     mdiAlertOutline
    //   );
    //   return;
    // }

    amount = luxToDusk(maxAmount);
  }

  onMount(() => {
    isGasValid = areValidGasSettings(gasPrice, gasLimit);
  });

  $: fee = gasLimit * gasPrice;
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
        disabled: amount === 0,
      }}
    >
      <ContractStatusesList {statuses} />
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
          bind:value={amount}
          type="number"
          max={luxToDusk(maxAmount)}
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
        {fee}
        limit={gasSettings.gasLimit}
        limitLower={gasLimits.gasLimitLower}
        limitUpper={gasLimits.gasLimitUpper}
        price={gasSettings.gasPrice}
        priceLower={gasLimits.gasPriceLower}
        on:gasSettings={setGasValues}
      />
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
        disabled: !isGasValid,
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
          value={formatter(amount)}
        />

        <GasSettings
          {formatter}
          {fee}
          limit={gasSettings.gasLimit}
          limitLower={gasLimits.gasLimitLower}
          limitUpper={gasLimits.gasLimitUpper}
          price={gasSettings.gasPrice}
          priceLower={gasLimits.gasPriceLower}
          on:gasSettings={setGasValues}
        />
      </div>
    </WizardStep>

    <!-- OPERATION RESULT STEP  -->
    <WizardStep step={2} {key} showNavigation={false}>
      <OperationResult
        errorMessage="Transaction failed"
        onBeforeLeave={resetOperation}
        operation={execute(amount, gasPrice, gasLimit)}
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
