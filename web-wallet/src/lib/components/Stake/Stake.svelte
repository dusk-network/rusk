<svelte:options immutable={true} />

<script>
  import { createEventDispatcher, onMount } from "svelte";
  import { fade } from "svelte/transition";
  import {
    mdiAlertOutline,
    mdiDatabaseArrowDownOutline,
    mdiDatabaseOutline,
  } from "@mdi/js";

  import { DOCUMENTATION_LINKS, MESSAGES } from "$lib/constants";
  import { areValidGasSettings, deductLuxFeeFrom } from "$lib/contracts";
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

  /** @type {StakeType} */
  export let flow;

  /** @type {(amount: number) => string} */
  export let formatter;

  /** @type {ContractGasSettings} */
  export let gasSettings;

  /** @type {number} */
  export let rewards;

  /** @type {bigint} */
  export let spendable;

  /** @type {number} */
  export let staked;

  /** @type {ContractStatus[]} */
  export let statuses;

  /** @type {boolean} */
  export let hideStakingNotice;

  /** @type {GasStoreContent} */
  export let gasLimits;

  /** @type {number} */
  export let minAllowedStake;

  /** @type {number} */
  let stakeAmount = {
    stake: minAllowedStake,
    unstake: staked,
    "withdraw-rewards": rewards,
  }[flow];

  /** @type {HTMLInputElement|null} */
  let stakeInput;

  /** @type {boolean} */
  let hideStakingNoticeNextTime = false;
  let isGasValid = false;

  let { gasLimit, gasPrice } = gasSettings;

  /** @type {Record<StakeType, string>} */
  const confirmLabels = {
    stake: "Stake",
    unstake: "Unstake",
    "withdraw-rewards": "Withdraw",
  };

  /** @type {Record<StakeType, string>} */
  const overviewLabels = {
    stake: "Amount",
    unstake: "Unstake Amount",
    "withdraw-rewards": "Withdraw Rewards",
  };

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

  onMount(() => {
    if (flow === "stake") {
      stakeAmount = Math.min(minStake, stakeAmount);
    }

    isGasValid = areValidGasSettings(gasPrice, gasLimit);
  });

  $: fee = gasLimit * gasPrice;
  $: maxSpendable = deductLuxFeeFrom(luxToDusk(spendable), fee);
  $: minStake =
    maxSpendable > 0
      ? Math.min(minAllowedStake, maxSpendable)
      : minAllowedStake;
  $: isStakeAmountValid =
    stakeAmount >= minStake && stakeAmount <= maxSpendable;
  $: totalLuxFee = fee + duskToLux(stakeAmount);
  $: isFeeWithinLimit = totalLuxFee <= spendable;
  $: isNextButtonDisabled =
    flow === "stake"
      ? !(isStakeAmountValid && isGasValid && isFeeWithinLimit)
      : false;

  function getWizardSteps() {
    if (flow === "stake") {
      return hideStakingNotice ? 3 : 4;
    }

    return 2;
  }

  function getStepperSteps() {
    if (flow === "stake") {
      return hideStakingNotice
        ? [{ label: "Amount" }, { label: "Overview" }, { label: "Done" }]
        : [
            { label: "Notice" },
            { label: "Amount" },
            { label: "Overview" },
            { label: "Done" },
          ];
    }

    return [{ label: "Overview" }, { label: "Done" }];
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

    if (stakeInput) {
      stakeInput.value = maxSpendable.toString();
    }

    stakeAmount = maxSpendable;
  }

  const steps = getStepperSteps();
  let activeStep = 0;
</script>

<div class="operation">
  <Wizard steps={getWizardSteps()} let:key>
    <div slot="stepper">
      <Stepper {activeStep} {steps} showStepLabelWhenInactive={false} />
    </div>

    {#if flow === "stake"}
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
          <WarningCard
            onSurface
            heading="Only stake if you have a node set up!"
          >
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
              > penalties, requiring me to unstake and restake.
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
            min={minStake}
            max={maxSpendable}
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
    {/if}

    <!-- OPERATION OVERVIEW STEP  -->
    <WizardStep
      step={flow === "stake" ? (hideStakingNotice ? 1 : 2) : 0}
      {key}
      backButton={{
        action: () => {
          if (flow === "stake") {
            activeStep--;
          } else {
            resetOperation();
          }
        },
        disabled: false,
      }}
      nextButton={{
        action: () => activeStep++,
        disabled: flow === "stake" ? stakeAmount === 0 : !isGasValid,
        icon: {
          path:
            flow === "stake" ? mdiDatabaseOutline : mdiDatabaseArrowDownOutline,
          position: "before",
        },
        label: confirmLabels[flow],
        variant: "primary",
      }}
    >
      <div in:fade|global class="operation__stake">
        <ContractStatusesList {statuses} />
        <Badge text="REVIEW TRANSACTION" variant="warning" />
        <StakeOverview
          label={overviewLabels[flow]}
          value={formatter(stakeAmount)}
        />

        {#if flow === "stake"}
          <GasFee {formatter} {fee} />
        {:else}
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
        {/if}
      </div>
    </WizardStep>

    <!-- OPERATION RESULT STEP  -->
    <WizardStep
      step={flow === "stake" ? (hideStakingNotice ? 2 : 3) : 1}
      {key}
      showNavigation={false}
    >
      <OperationResult
        errorMessage="Transaction failed"
        onBeforeLeave={resetOperation}
        operation={flow === "stake"
          ? execute(duskToLux(stakeAmount), gasPrice, gasLimit)
          : execute(gasPrice, gasLimit)}
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
