<svelte:options immutable={true} />

<script>
  import { fade } from "svelte/transition";
  import { createEventDispatcher, onMount } from "svelte";
  import { mdiArrowUpBoldBoxOutline, mdiWalletOutline } from "@mdi/js";

  import { areValidGasSettings, deductLuxFeeFrom } from "$lib/contracts";
  import { duskToLux, luxToDusk } from "$lib/dusk/currency";
  import { validateAddress } from "$lib/dusk/string";
  import { logo } from "$lib/dusk/icons";
  import {
    Badge,
    Button,
    Icon,
    Textbox,
    Wizard,
    WizardStep,
  } from "$lib/dusk/components";
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

  /** @type {number} */
  export let spendable;

  /** @type {ContractStatus[]} */
  export let statuses;

  /** @type {import("$lib/stores/stores").GasStoreContent} */
  export let gasLimits;

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
  let isGasValid = false;

  let { gasLimit, gasPrice } = gasSettings;

  const minAmount = 0.000000001;

  const dispatch = createEventDispatcher();
  const resetOperation = () => dispatch("operationChange", "");

  onMount(() => {
    amountInput = document.querySelector(".operation__input-field");
    isGasValid = areValidGasSettings(gasPrice, gasLimit);
  });

  $: luxFee = gasLimit * gasPrice;
  $: fee = formatter(luxToDusk(luxFee));
  $: maxSpendable = deductLuxFeeFrom(spendable, luxFee);
  $: isAmountValid = amount >= minAmount && amount <= maxSpendable;
  $: totalLuxFee = luxFee + duskToLux(amount);
  $: isFeeWithinLimit = totalLuxFee <= duskToLux(spendable);
  $: isNextButtonDisabled = !(isAmountValid && isGasValid && isFeeWithinLimit);

  $: addressValidationResult = validateAddress(address);
</script>

<div class="operation">
  <Wizard steps={4} let:key>
    <WizardStep
      step={0}
      {key}
      backButton={{
        action: resetOperation,
        disabled: false,
      }}
      nextButton={{ disabled: isNextButtonDisabled }}
    >
      <div in:fade|global class="operation__send">
        <ContractStatusesList items={statuses} />
        <div class="operation__send-amount operation__space-between">
          <p>Enter amount:</p>
          <Button
            size="small"
            variant="tertiary"
            on:click={() => {
              if (amountInput) {
                amountInput.value = maxSpendable.toString();
              }

              amount = maxSpendable;
            }}
            text="USE MAX"
          />
        </div>

        <div class="operation__send-amount operation__input">
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
      step={1}
      {key}
      nextButton={{ disabled: !addressValidationResult.isValid }}
    >
      <div in:fade|global class="operation__send">
        <ContractStatusesList items={statuses} />

        <div class="operation__send-amount operation__space-between">
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
          className="operation__send-address
						{!addressValidationResult.isValid ? 'operation__send-address--invalid' : ''}"
          type="multiline"
          bind:value={address}
        />
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
      step={2}
      {key}
      nextButton={{
        icon: { path: mdiArrowUpBoldBoxOutline, position: "before" },
        label: "SEND",
        variant: "secondary",
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
        onBeforeLeave={resetOperation}
        operation={execute(address, amount, gasPrice, gasLimit)}
        pendingMessage="Processing transaction"
        successMessage="Transaction completed"
      >
        <svelte:fragment slot="success-content" let:result={hash}>
          {#if hash}
            <AppAnchorButton
              href={`https://explorer.dusk.network/transactions/transaction?id=${hash}`}
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
    &__review-address {
      background-color: transparent;
      border: 1px solid var(--primary-color);
      border-radius: 1.5em;
      padding: 0.75em 1em;
      width: 100%;
      line-break: anywhere;
    }

    &__review-transaction {
      display: flex;
      flex-direction: column;
      gap: 0.625em;
    }

    &__review-amount {
      justify-content: flex-start;
    }

    &__send {
      display: flex;
      flex-direction: column;
      gap: 1.2em;
    }

    &__send-amount {
      display: flex;
      align-items: center;
      width: 100%;
    }

    &__space-between {
      justify-content: space-between;
    }

    &__input {
      column-gap: var(--default-gap);
    }

    :global(&__input &__input-field) {
      width: 100%;
      padding: 0.5em 1em;
    }

    :global(&__input-field:invalid) {
      color: var(--error-color);
    }

    :global(&__send-address) {
      width: 100%;
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

  :global(.operation__send-address--invalid) {
    color: var(--error-color);
  }
</style>
