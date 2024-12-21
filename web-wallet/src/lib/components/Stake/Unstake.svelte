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
    Stepper,
    Wizard,
    WizardStep,
  } from "$lib/dusk/components";
  import { GasSettings, OperationResult } from "$lib/components";

  import StakeOverview from "./StakeOverview.svelte";

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
  const amount = luxToDusk(maxAmount);

  const steps = [{ label: "Overview" }, { label: "Done" }];

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

  onMount(() => {
    isGasValid = areValidGasSettings(gasPrice, gasLimit);
  });

  $: fee = gasLimit * gasPrice;
</script>

<div class="operation">
  <Wizard steps={2} let:key>
    <div slot="stepper">
      <Stepper {activeStep} {steps} />
    </div>

    <!-- OPERATION OVERVIEW STEP  -->
    <WizardStep
      step={0}
      {key}
      backButton={{
        action: () => resetOperation(),
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
    <WizardStep step={1} {key} showNavigation={false}>
      <OperationResult
        errorMessage="Transaction failed"
        onBeforeLeave={resetOperation}
        operation={execute(gasPrice, gasLimit)}
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
  }
</style>
