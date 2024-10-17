<svelte:options immutable={true} />

<script>
  import { fade } from "svelte/transition";
  import { onMount } from "svelte";
  import {
    mdiAlertOutline,
    mdiArrowUpBoldBoxOutline,
    mdiWalletOutline,
  } from "@mdi/js";
  import { areValidGasSettings, deductLuxFeeFrom } from "$lib/contracts";
  import { duskToLux, luxToDusk } from "$lib/dusk/currency";
  import { validateAddress } from "$lib/dusk/string";
  import { logo } from "$lib/dusk/icons";
  import {
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
    AppAnchorButton,
    ContractStatusesList,
    GasFee,
    GasSettings,
    OperationResult,
    ScanQR,
  } from "$lib/components";

  /** @type {(to: string, amount: number, gasPrice: number, gasLimit: number) => Promise<string>} */
  export let execute;

  /** @type {(amount: number) => string} */
  export let formatter;

  /** @type {ContractGasSettings} */
  export let gasSettings;

  /** @type {bigint} */
  export let spendable;

  /** @type {ContractStatus[]} */
  export let statuses;

  /** @type {GasStoreContent} */
  export let gasLimits;

  /** @type {boolean} */
  export let enableAllocateButton = false;

  /** @type {number} */
  let amount = 1;

  /** @type {string} */
  let address = "";

  /** @type {import("qr-scanner").default} */
  let scanner;

  /** @type {import("..").ScanQR} */
  let scanQrComponent;

  /** @type {HTMLInputElement | null} */
  let amountInput;

  /** @type {boolean} */
  let isNextButtonDisabled = false;

  /** @type {boolean} */
  let isGasValid = false;

  let { gasLimit, gasPrice } = gasSettings;

  const minAmount = 0.000000001;
  const steps = [
    { label: "Address" },
    { label: "Amount" },
    { label: "Review" },
    { label: "Done" },
  ];

  onMount(() => {
    amountInput = document.querySelector(".operation__input-field");
    isGasValid = areValidGasSettings(gasPrice, gasLimit);
  });

  $: luxFee = gasLimit * gasPrice;
  $: fee = formatter(luxToDusk(luxFee));
  $: maxSpendable = deductLuxFeeFrom(luxToDusk(spendable), luxFee);
  $: isAmountValid = amount >= minAmount && amount <= maxSpendable;
  $: totalLuxFee = luxFee + (amount ? duskToLux(amount) : 0);
  $: isFeeWithinLimit = BigInt(totalLuxFee) <= spendable;
  $: isNextButtonDisabled = !(isAmountValid && isGasValid && isFeeWithinLimit);
  $: addressValidationResult = validateAddress(address);

  function setMaxAmount() {
    if (!isGasValid) {
      toast("error", "Please set valid gas settings first", mdiAlertOutline);
      return;
    }

    if (spendable < BigInt(luxFee)) {
      toast(
        "error",
        "You don't have enough DUSK to cover the transaction fee",
        mdiAlertOutline
      );
      return;
    }

    if (amountInput) {
      amountInput.value = maxSpendable.toString();
    }

    amount = maxSpendable;
  }

  let activeStep = 0;
</script>

<div class="operation">
  <Wizard steps={steps.length} let:key>
    <div slot="stepper">
      <Stepper {activeStep} {steps} showStepLabelWhenInactive={false} />
    </div>
    <WizardStep
      step={0}
      {key}
      backButton={{
        disabled: false,
        href: "/dashboard",
        isAnchor: true,
      }}
      nextButton={{
        action: () => {
          activeStep = 1;
        },
        disabled: !addressValidationResult.isValid,
      }}
    >
      <div in:fade|global class="operation__send">
        <div class="operation__address-wrapper">
          <p>Enter address:</p>
          <Button
            disabled={!scanner}
            size="small"
            on:click={() => {
              scanQrComponent.startScan();
            }}
            text="SCAN QR"
          />
        </div>
        <Textbox
          required
          className={`operation__send-address ${!addressValidationResult.isValid ? "operation__send-address--invalid" : ""}`}
          type="multiline"
          bind:value={address}
        />
        <ContractStatusesList items={statuses}>
          {#if enableAllocateButton}
            <AppAnchorButton
              className="allocate-button"
              href="/dashboard/allocate"
              text="Shield more DUSK"
              variant="tertiary"
            />
          {/if}
        </ContractStatusesList>
        <ScanQR
          bind:this={scanQrComponent}
          bind:scanner
          on:scan={(event) => {
            address = event.detail;
          }}
        />
      </div>
    </WizardStep>
    <WizardStep
      step={1}
      {key}
      backButton={{
        action: () => {
          activeStep = 0;
        },
      }}
      nextButton={{
        action: () => {
          activeStep = 2;
        },
        disabled: isNextButtonDisabled,
      }}
    >
      <div in:fade|global class="operation__send">
        <ContractStatusesList items={statuses}>
          {#if enableAllocateButton}
            <AppAnchorButton
              className="allocate-button"
              href="/dashboard/allocate"
              text="Shield more DUSK"
              variant="tertiary"
            />
          {/if}
        </ContractStatusesList>

        <div class="operation__amount-wrapper">
          <p>Enter amount:</p>
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
            required
            type="number"
            min={minAmount}
            max={maxSpendable}
            step="0.000000001"
          />
          <Icon
            data-tooltip-id="main-tooltip"
            data-tooltip-text="DUSK"
            path={logo}
          />
        </div>

        <GasSettings
          {fee}
          limit={gasSettings.gasLimit}
          limitLower={gasLimits.gasLimitLower}
          limitUpper={gasLimits.gasLimitUpper}
          price={gasSettings.gasPrice}
          priceLower={gasLimits.gasPriceLower}
          on:gasSettings={(event) => {
            isGasValid = areValidGasSettings(
              event.detail.price,
              event.detail.limit
            );

            if (isGasValid) {
              gasPrice = event.detail.price;
              gasLimit = event.detail.limit;
            }
          }}
        />
      </div>
    </WizardStep>
    <WizardStep
      step={2}
      {key}
      backButton={{
        action: () => {
          activeStep = 1;
        },
      }}
      nextButton={{
        action: () => {
          activeStep = 3;
        },
        icon: { path: mdiArrowUpBoldBoxOutline, position: "before" },
        label: "SEND",
        variant: "primary",
      }}
    >
      <div in:fade|global class="operation__send">
        <ContractStatusesList items={statuses} />

        <Badge
          className="operation__review-notice"
          text="REVIEW TRANSACTION"
          variant="warning"
        />

        <dl class="operation__review-transaction">
          <dt class="review-transaction__label">
            <Icon path={mdiArrowUpBoldBoxOutline} />
            <span>Amount:</span>
          </dt>
          <dd class="review-transaction__value operation__review-amount">
            <span>{formatter(amount)}</span>
            <Icon
              className="dusk-amount__icon"
              path={logo}
              data-tooltip-id="main-tooltip"
              data-tooltip-text="DUSK"
            />
          </dd>
        </dl>

        <dl class="operation__review-transaction">
          <dt class="review-transaction__label">
            <Icon path={mdiWalletOutline} />
            <span>To:</span>
          </dt>
          <dd class="operation__review-address">
            <span>{address}</span>
          </dd>
        </dl>

        <GasFee {fee} />
      </div>
    </WizardStep>
    <WizardStep step={3} {key} showNavigation={false}>
      <OperationResult
        errorMessage="Transaction failed"
        operation={execute(address, amount, gasPrice, gasLimit)}
        pendingMessage="Processing transaction"
        successMessage="Transaction completed"
      >
        <svelte:fragment slot="success-content" let:result={hash}>
          {#if hash}
            <AppAnchorButton
              href={`https://explorer.dusk.network/transactions/transaction?id=${hash}`}
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
    &__send {
      display: flex;
      flex-direction: column;
      gap: 1.2em;
    }
    &__review-address {
      background-color: transparent;
      border: 1px solid var(--primary-color);
      border-radius: 1.5em;
      padding: 0.75em 1em;
      width: 100%;
      line-break: anywhere;
    }

    &__address-wrapper,
    &__amount-wrapper,
    &__input-wrapper {
      display: flex;
      justify-content: space-between;
      align-items: center;
      width: 100%;
    }

    &__input-wrapper {
      column-gap: var(--default-gap);
    }

    &__review-transaction {
      display: flex;
      flex-direction: column;
      gap: 0.625em;
    }

    &__review-amount {
      justify-content: flex-start;
    }

    :global(&__review-notice) {
      text-align: center;
    }

    :global(.dusk-amount__icon) {
      width: 1em;
      height: 1em;
      flex-shrink: 0;
    }
  }

  .review-transaction__label,
  .review-transaction__value {
    display: inline-flex;
    align-items: center;
    gap: var(--small-gap);
  }

  .review-transaction__value {
    font-weight: bold;
  }

  :global(.dusk-textbox.operation__send-address) {
    resize: vertical;
    min-height: 5em;
    max-height: 10em;
  }

  :global(.dusk-textbox.operation__send-address--invalid) {
    color: var(--error-color);
  }

  :global(.allocate-button) {
    width: 100%;
  }
</style>
