<svelte:options immutable={true} />

<script>
  import { fade } from "svelte/transition";
  import { createEventDispatcher, onMount } from "svelte";
  import {
    mdiAlertOutline,
    mdiArrowUpBoldBoxOutline,
    mdiWalletOutline,
  } from "@mdi/js";
  import { areValidGasSettings } from "$lib/contracts";
  import { duskToLux, luxToDusk } from "$lib/dusk/currency";
  import { getAddressInfo } from "$lib/dusk/string";
  import { logo } from "$lib/dusk/icons";
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
  import { toast } from "$lib/dusk/components/Toast/store";
  import {
    Banner,
    ContractStatusesList,
    GasFee,
    GasSettings,
    OperationResult,
    ScanQR,
  } from "$lib/components";
  import { MESSAGES } from "$lib/constants";

  /** @type {(to: string, amount: bigint, gasPrice: bigint, gasLimit: bigint) => Promise<string>} */
  export let execute;

  /** @type {(amount: number) => string} */
  export let formatter;

  /** @type {ContractGasSettings} */
  export let gasSettings;

  /** @type {bigint} */
  export let availableBalance;

  /** @type {ContractStatus[]} */
  export let statuses;

  /** @type {GasStoreContent} */
  export let gasLimits;

  /** @type {boolean} */
  export let enableMoonlightTransactions = false;

  /** @type {string} */
  export let shieldedAddress;

  /** @type {string} */
  export let publicAddress;

  /** @type {number} */
  let sendAmount = 1;

  /** @type {string} */
  let sendToAddress = "";

  /** @type {import("qr-scanner").default} */
  let scanner;

  /** @type {import("..").ScanQR} */
  let scanQrComponent;

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
  const dispatch = createEventDispatcher();

  onMount(() => {
    isGasValid = areValidGasSettings(gasPrice, gasLimit);
  });

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

    sendAmount = luxToDusk(maxSpendableAmount);
  }

  let activeStep = 0;

  /**
   * Validates an address/account depending on moonlight transactions being enabled.
   *
   * Note. This function can be removed when the VITE_FEATURE_MOONLIGHT_TRANSACTIONS flag is removed.
   *
   * @param {{isValid: boolean, type?: "address" | "account", isSelfReferential? : boolean}} addressInfo
   */
  function isValid(addressInfo) {
    return !addressInfo.isValid
      ? true
      : isMoonlightTransaction
        ? !enableMoonlightTransactions
        : false;
  }

  $: sendAmountInLux = sendAmount ? duskToLux(sendAmount) : 0n;

  // Calculate the maximum gas fee based on the gas limit and gas price.
  $: maxGasFee = gasLimit * gasPrice;

  // Check if the available balance is sufficient to cover the max gas fee.
  // This is a prerequisite for any transaction.
  $: isBalanceSufficientForGas = availableBalance >= maxGasFee;

  // Determine the maximum amount spendable for the transfer.
  // If the available balance is less than the max gas fee, set it to 0n to avoid negative values.
  $: maxSpendableAmount =
    availableBalance >= maxGasFee ? availableBalance - maxGasFee : 0n;

  // Validate that the send amount is within allowable limits:
  // - At least the minimum send requirement.
  // - At most the maximum spendable amount (after accounting for maximum gas fees).
  $: isSendAmountValid =
    sendAmountInLux >= minAmount && sendAmountInLux <= maxSpendableAmount;

  // Calculate the total amount for the transaction, which includes:
  // - The maximum gas fee.
  // - The user-specified amount to send (converted to Lux).
  $: totalAmount = maxGasFee + sendAmountInLux;

  // Validate that the total amount is within the user's available balance.
  $: isTotalAmountWithinAvailableBalance = totalAmount <= availableBalance;

  $: isNextButtonDisabled = !(
    isSendAmountValid &&
    isGasValid &&
    isTotalAmountWithinAvailableBalance &&
    isBalanceSufficientForGas
  );

  $: addressInfo = getAddressInfo(
    sendToAddress,
    shieldedAddress,
    publicAddress
  );

  $: isMoonlightTransaction = addressInfo.type === "account";

  $: if (addressInfo.type) {
    dispatch("keyChange", {
      type: addressInfo.type,
    });
  }
</script>

<div class="operation">
  <Wizard steps={steps.length} let:key>
    <div slot="stepper">
      <Stepper {activeStep} {steps} showStepLabelWhenInactive={false} />
    </div>
    <!-- Address Step -->
    <WizardStep
      step={0}
      {key}
      backButton={{
        disabled: false,
        href: "/dashboard",
        isAnchor: true,
      }}
      nextButton={{
        action: () => activeStep++,
        disabled: isValid(addressInfo),
      }}
    >
      <div in:fade|global class="operation__send">
        <ContractStatusesList {statuses} />
        <div class="operation__address-wrapper">
          <p>Address:</p>
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
          className={`operation__send-address ${!addressInfo.isValid ? "operation__send-address--invalid" : ""}`}
          type="multiline"
          bind:value={sendToAddress}
        />
        {#if addressInfo.type === "account"}
          <Banner
            title="Public account detected"
            variant={enableMoonlightTransactions ? "info" : "warning"}
          >
            {#if enableMoonlightTransactions}
              <p>
                This transaction will be public and sent from your <strong
                  >public</strong
                > account.
              </p>
            {:else}
              <p>Public transactions are currently unavailable.</p>
            {/if}
          </Banner>
        {/if}
        <ScanQR
          bind:this={scanQrComponent}
          bind:scanner
          on:scan={(event) => {
            sendToAddress = event.detail;
          }}
        />
        {#if addressInfo.isSelfReferential}
          <Banner
            variant="warning"
            title="Self-referential transaction detected"
          >
            <p>
              You are attempting to initiate a transaction with your own wallet
              address as both the sender and the receiver. Self-referential
              transactions may not have meaningful purpose and will incur gas
              fees.
            </p>
          </Banner>
        {/if}
      </div>
    </WizardStep>
    <!-- Amount Step -->
    <WizardStep
      step={1}
      {key}
      backButton={{
        action: () => activeStep--,
      }}
      nextButton={{
        action: () => activeStep++,
        disabled: isNextButtonDisabled,
      }}
    >
      <div in:fade|global class="operation__send">
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
            bind:value={sendAmount}
            required
            type="number"
            min={minAmount}
            max={luxToDusk(maxSpendableAmount)}
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
        {#if !isBalanceSufficientForGas}
          <Banner variant="error" title="Insufficient balance for gas fees">
            <p>
              Your current balance is too low to cover the required gas fees for
              this transaction. Please deposit additional funds or reduce the
              gas limit.
            </p>
          </Banner>
        {:else if sendAmountInLux > maxSpendableAmount}
          <Banner variant="error" title="Amount exceeds available balance">
            <p>
              The amount you are trying to transfer exceeds the spendable
              balance after accounting for gas fees. Please reduce the amount.
            </p>
          </Banner>
        {/if}
      </div>
    </WizardStep>
    <!-- Review Step -->
    <WizardStep
      step={2}
      {key}
      backButton={{
        action: () => activeStep--,
      }}
      nextButton={{
        action: () => activeStep++,
        icon: { path: mdiArrowUpBoldBoxOutline, position: "before" },
        label: "SEND",
        variant: "primary",
      }}
    >
      <div in:fade|global class="operation__send">
        <ContractStatusesList {statuses} />

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
            <span>{formatter(sendAmount)}</span>
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
            <span>{sendToAddress}</span>
          </dd>
        </dl>

        <GasFee {formatter} fee={maxGasFee} />
      </div>
    </WizardStep>
    <!-- Result Step -->
    <WizardStep step={3} {key} showNavigation={false}>
      <OperationResult
        errorMessage="Transaction failed"
        operation={execute(sendToAddress, sendAmountInLux, gasPrice, gasLimit)}
        pendingMessage="Processing transaction"
        successMessage="Transaction created"
      >
        <svelte:fragment slot="success-content" let:result={hash}>
          <p>{MESSAGES.TRANSACTION_CREATED}</p>
          {#if hash}
            <AnchorButton
              href={`/explorer/transactions/transaction?id=${hash}`}
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
