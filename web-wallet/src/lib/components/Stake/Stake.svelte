<svelte:options immutable={true} />

<script>
  import { createEventDispatcher, onMount } from "svelte";
  import { fade } from "svelte/transition";
  import { mdiAlertOutline, mdiDatabaseOutline } from "@mdi/js";

  import { DOCUMENTATION_LINKS, MESSAGES } from "$lib/constants";
  import { areValidGasSettings } from "$lib/contracts";
  import { duskToLux, luxToDusk } from "$lib/dusk/currency";
  import { logo } from "$lib/dusk/icons";

  import {
    Agreement,
    AnchorButton,
    Badge,
    Button,
    Icon,
    Stepper,
    Textbox,
    Wizard,
    WizardStep,
  } from "$lib/dusk/components";
  import { toast } from "$lib/dusk/components/Toast/store";
  import {
    AppAnchor,
    ContractStatusesList,
    GasFee,
    GasSettings,
    OperationResult,
  } from "$lib/components";

  import { WarningCard } from "$lib/containers/Cards";

  import StakeOverview from "./StakeOverview.svelte";

  /** @type {(...args: any[]) => Promise<string>} */
  export let execute;

  /** @type {(amount: number) => string} */
  export let formatter;

  /** @type {GasStoreContent} */
  export let gasLimits;

  /** @type {ContractGasSettings} */
  export let gasSettings;

  /** @type {boolean} */
  export let hideStakingNotice;

  /** @type {bigint} */
  export let minAllowedStake;

  /** @type {bigint} */
  export let spendable;

  /** @type {ContractStatus[]} */
  export let statuses;

  let activeStep = 0;
  let { gasLimit, gasPrice } = gasSettings;
  let hideStakingNoticeNextTime = false;
  let isGasValid = false;

  /**
   * We are forced to keep `stakeAmount`
   * as number if we want to use
   * Svelte's binding.
   */
  let stakeAmount = luxToDusk(minAllowedStake);

  const steps = getStepperSteps();

  const dispatch = createEventDispatcher();
  const resetOperation = () => dispatch("operationChange", "");
  const suppressStakingNotice = () => dispatch("suppressStakingNotice");

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

  function getStepperSteps() {
    return hideStakingNotice
      ? [{ label: "Amount" }, { label: "Overview" }, { label: "Done" }]
      : [
          { label: "Notice" },
          { label: "Amount" },
          { label: "Overview" },
          { label: "Done" },
        ];
  }

  function setMaxAmount() {
    if (!isGasValid) {
      toast("error", "Please set valid gas settings first", mdiAlertOutline);
      return;
    }

    if (spendable < fee) {
      toast(
        "error",
        "You don't have enough DUSK to cover the transaction fee",
        mdiAlertOutline
      );
      return;
    }

    stakeAmount = luxToDusk(maxSpendable);
  }

  onMount(() => {
    stakeAmount = Math.min(luxToDusk(minStake), stakeAmount);
    isGasValid = areValidGasSettings(gasPrice, gasLimit);
  });

  // Derived values
  $: fee = gasLimit * gasPrice;
  $: maxSpendable = spendable - fee;
  $: minStake =
    maxSpendable > 0n && minAllowedStake > maxSpendable
      ? maxSpendable
      : minAllowedStake;
  $: stakeAmountInLux = stakeAmount ? duskToLux(stakeAmount) : 0n;
  $: isStakeAmountValid =
    stakeAmountInLux >= minStake && stakeAmountInLux <= maxSpendable;
  $: totalLuxFee = fee + stakeAmountInLux;
  $: isFeeWithinLimit = totalLuxFee <= spendable;
  $: isNextButtonDisabled = !(
    isStakeAmountValid &&
    isGasValid &&
    isFeeWithinLimit
  );
</script>

<div class="operation">
  <Wizard steps={hideStakingNotice ? 3 : 4} let:key>
    <div slot="stepper">
      <Stepper {activeStep} {steps} showStepLabelWhenInactive={false} />
    </div>

    {#if !hideStakingNotice}
      <!-- STAKING NOTICE STEP -->
      <WizardStep
        step={0}
        {key}
        backButton={{
          action: resetOperation,
          disabled: false,
        }}
        nextButton={{
          action: () => {
            activeStep++;
            if (hideStakingNoticeNextTime) {
              suppressStakingNotice();
            }
          },
          icon: null,
          label: "Agree",
          variant: "primary",
        }}
      >
        <Badge text="WARNING" variant="warning" />
        <WarningCard onSurface heading="Only stake if you have a node set up!">
          <p class="staking-warning">
            I understand that I have set up a node properly, as described <AppAnchor
              class="staking-warning__step-node-setup-link"
              rel="noopener noreferrer"
              target="_blank"
              href={DOCUMENTATION_LINKS.RUN_A_PROVISIONER}>HERE</AppAnchor
            >, and that, if not done correctly, I may be subject to <AppAnchor
              class="staking-warning__step-node-setup-link"
              href={DOCUMENTATION_LINKS.SLASHING}
              rel="noopener noreferrer"
              target="_blank">soft-slashing</AppAnchor
            > penalties, requiring me to unstake and stake again.
          </p>

          <Agreement
            className="staking-warning__agreement"
            name="staking-warning"
            label="Don't show this step again."
            bind:checked={hideStakingNoticeNextTime}
          />
        </WarningCard>
      </WizardStep>
    {/if}

    <!-- ENTER STAKING AMOUNT STEP -->
    <WizardStep
      step={hideStakingNotice ? 0 : 1}
      {key}
      backButton={{
        action: () => {
          if (hideStakingNotice) {
            resetOperation();
          } else {
            activeStep--;
          }
        },
        disabled: false,
      }}
      nextButton={{
        action: () => activeStep++,
        disabled: isNextButtonDisabled,
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
          bind:value={stakeAmount}
          type="number"
          min={luxToDusk(minStake)}
          max={luxToDusk(maxSpendable)}
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
      step={hideStakingNotice ? 1 : 2}
      {key}
      backButton={{
        action: () => activeStep--,
        disabled: false,
      }}
      nextButton={{
        action: () => activeStep++,
        disabled: stakeAmount === 0,
        icon: {
          path: mdiDatabaseOutline,
          position: "before",
        },
        label: "Stake",
        variant: "primary",
      }}
    >
      <div in:fade|global class="operation__stake">
        <ContractStatusesList {statuses} />
        <Badge text="REVIEW TRANSACTION" variant="warning" />
        <StakeOverview label="Amount" value={formatter(stakeAmount)} />
        <GasFee {formatter} {fee} />
      </div>
    </WizardStep>

    <!-- OPERATION RESULT STEP  -->
    <WizardStep step={hideStakingNotice ? 2 : 3} {key} showNavigation={false}>
      <OperationResult
        errorMessage="Transaction failed"
        onBeforeLeave={resetOperation}
        operation={execute(stakeAmountInLux, gasPrice, gasLimit)}
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
    &__stake {
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

  .staking-warning {
    margin-bottom: 1em;
  }

  :global(.staking-warning__step-node-setup-link) {
    color: var(--secondary-color-variant-dark);
  }

  :global(.staking-warning__agreement) {
    align-items: baseline;
  }
</style>
