<svelte:options immutable={true} />

<script>
  import { fade } from "svelte/transition";
  import { onMount } from "svelte";
  import {
    mdiArrowUpBoldBoxOutline,
    mdiShieldLock,
    mdiShieldLockOpenOutline,
  } from "@mdi/js";
  import { areValidGasSettings } from "$lib/contracts";
  import { duskToLux, luxToDusk } from "$lib/dusk/currency";
  import { calculateAdaptiveCharCount, middleEllipsis } from "$lib/dusk/string";
  import { logo } from "$lib/dusk/icons";
  import {
    AnchorButton,
    Badge,
    Icon,
    Textbox,
    Wizard,
    WizardStep,
  } from "$lib/dusk/components";
  import { GasFee, GasSettings, OperationResult } from "$lib/components";
  import { walletStore } from "$lib/stores";
  import { Gas } from "$lib/vendor/w3sper.js/src/mod";
  import { MESSAGES } from "$lib/constants";
  import Banner from "../Banner/Banner.svelte";

  /** @type {(amount: number) => string} */
  export let formatter;

  /** @type {ContractGasSettings} */
  export let gasSettings;

  /** @type {GasStoreContent} */
  export let gasLimits;

  /** @type {string} */
  export let shieldedAddress;

  /** @type {string} */
  export let unshieldedAddress;

  /** @type {bigint} */
  export let shieldedBalance;

  /** @type {bigint} */
  export let unshieldedBalance;

  // @ts-ignore
  function handleBalanceChange(event) {
    const value = parseFloat(event.target.value);

    if (isNaN(value) || value < 0) {
      hasInvalidInput = true;
      return;
    }

    hasInvalidInput = false;

    const isShieldedAmount = event.target.name === "shielded-amount";
    shielded = isShieldedAmount
      ? duskToLux(value)
      : totalBalance - duskToLux(value);
  }

  async function allocate() {
    const gas = new Gas({ limit: gasLimit, price: gasPrice });

    if (difference !== 0n) {
      const transactionInfo = isShielding
        ? await walletStore.shield(difference - fee, gas)
        : await walletStore.unshield(-difference - fee, gas);

      return transactionInfo.hash;
    }

    // We shouldn't end up in this case,
    // as the next button should be disabled
    return Promise.resolve();
  }

  onMount(() => {
    isGasValid = areValidGasSettings(gasPrice, gasLimit);

    const resizeObserver = new ResizeObserver((entries) => {
      const entry = entries[0];

      screenWidth = entry.contentRect.width;
    });

    resizeObserver.observe(document.body);

    return () => resizeObserver.disconnect();
  });

  /** @type {boolean} */
  let isNextButtonDisabled = false;

  /** @type {boolean} */
  let isGasValid = false;

  let { gasLimit, gasPrice } = gasSettings;

  /** @type {number} */
  let screenWidth = window.innerWidth;

  let hasInvalidInput = false;
  let hasEnoughFunds = true;

  // Constant total
  const totalBalance = shieldedBalance + unshieldedBalance;

  // Used to keep the difference between the initial shielded balance and the current one
  const initialShielded = shieldedBalance;

  // Internal state of the balances
  let shielded = shieldedBalance;
  $: unshielded = totalBalance - shielded;

  // Derived number states for UI Inputs
  $: shieldedNumber = luxToDusk(shielded);
  $: unshieldedNumber = luxToDusk(unshielded);

  $: isShielding = difference > 0n;
  $: isUnshielding = !isShielding;

  $: fee = gasLimit * gasPrice;
  $: difference = shielded - initialShielded;

  $: hasEnoughFunds = isUnshielding
    ? shieldedBalance - difference - fee >= 0n
    : unshieldedBalance + difference - fee >= 0n;

  $: isNextButtonDisabled =
    !hasEnoughFunds ||
    !isGasValid ||
    difference === 0n || // No change in balance
    shielded < 0n || // Shielded balance is negative
    unshielded < 0n || // Unshielded balance is negative
    shielded + unshielded > totalBalance ||
    hasInvalidInput;
</script>

<div class="operation">
  <Wizard steps={3} let:key>
    <WizardStep
      step={0}
      {key}
      backButton={{
        disabled: false,
        href: "/dashboard",
        isAnchor: true,
      }}
      nextButton={{ disabled: isNextButtonDisabled }}
    >
      <div in:fade|global class="operation__allocate">
        <p>
          Edit the value to change the allocation of your Dusk between your
          shielded and public accounts.
        </p>

        <fieldset class="operation__fieldset">
          <p class="operation__label">Shielded</p>

          <div class="operation__address-wrapper">
            <Icon path={mdiShieldLock} />
            {middleEllipsis(
              shieldedAddress,
              calculateAdaptiveCharCount(screenWidth, 320, 640, 5, 20)
            )}
          </div>

          <div class="operation__input-wrapper">
            <Textbox
              className="operation__input-field"
              value={shieldedNumber}
              required
              type="number"
              step="0.000000001"
              max={luxToDusk(totalBalance)}
              min="0"
              on:input={handleBalanceChange}
              name="shielded-amount"
            />
            <Icon
              data-tooltip-id="main-tooltip"
              data-tooltip-text="DUSK"
              path={logo}
            />
          </div>

          <hr class="glyph" />

          <p class="operation__label">Public</p>

          <div class="operation__address-wrapper">
            <Icon path={mdiShieldLockOpenOutline} />
            {middleEllipsis(
              unshieldedAddress,
              calculateAdaptiveCharCount(screenWidth, 320, 640, 5, 20)
            )}
          </div>

          <div class="operation__input-wrapper">
            <Textbox
              className="operation__input-field"
              value={unshieldedNumber}
              required
              type="number"
              max={luxToDusk(totalBalance)}
              min="0"
              step="0.000000001"
              id="unshielded-amount"
              on:input={handleBalanceChange}
              name="unshielded-amount"
            />
            <Icon
              data-tooltip-id="main-tooltip"
              data-tooltip-text="DUSK"
              path={logo}
            />
          </div>
        </fieldset>

        <GasSettings
          {formatter}
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

        {#if !hasEnoughFunds}
          <Banner title="Insufficient Funds" variant="error">
            <p>
              Your balance is too low to cover the allocation fees. Please
              adjust your transaction or add more funds to proceed.
            </p>
          </Banner>
        {/if}
      </div>
    </WizardStep>
    <WizardStep
      step={1}
      {key}
      nextButton={{
        icon: { path: mdiArrowUpBoldBoxOutline, position: "before" },
        label: "SEND",
        variant: "primary",
      }}
    >
      <div in:fade|global class="operation__allocate">
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
            <span>
              {isShielding
                ? `${formatter(luxToDusk(unshieldedBalance - unshielded - fee))} DUSK`
                : `${formatter(luxToDusk(shieldedBalance - shielded - fee))} DUSK`}
            </span>
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
            <Icon
              path={isShielding ? mdiShieldLockOpenOutline : mdiShieldLock}
            />
            <span>From</span>
          </dt>
          <dd class="operation__review-address">
            <span>
              {isShielding ? unshieldedAddress : shieldedAddress}
            </span>
          </dd>
        </dl>
        <dl class="operation__review-transaction">
          <dt class="review-transaction__label">
            <Icon
              path={isUnshielding ? mdiShieldLockOpenOutline : mdiShieldLock}
            />
            <span>To</span>
          </dt>
          <dd class="operation__review-address">
            <span>
              {isUnshielding ? unshieldedAddress : shieldedAddress}
            </span>
          </dd>
        </dl>
        <GasFee {formatter} {fee} />
        <Banner title="Fee Details" variant="info">
          <p>
            The fee will be deducted from your <b
              >{isUnshielding ? "shielded" : "public"}</b
            > balance, with the maximum estimated fee reserved before allocation.
            This guarantees sufficient coverage for the transaction.
          </p>
        </Banner>
      </div>
    </WizardStep>
    <WizardStep step={2} {key} showNavigation={false}>
      <OperationResult
        errorMessage="Transaction failed"
        operation={allocate()}
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
    &__fieldset {
      display: flex;
      padding: 1em 1.25em;
      flex-direction: column;
      justify-content: center;
      align-items: flex-start;
      gap: var(--medium-gap);
      align-self: stretch;

      border-radius: var(--fieldset-border-radius);
      background: var(--fieldset-background-color);
    }

    &__address-wrapper,
    &__input-wrapper {
      display: flex;
      align-items: center;
      width: 100%;
    }

    &__address-wrapper {
      font-family: var(--mono-font-family);
      justify-content: space-between;
    }

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

    &__allocate {
      display: flex;
      flex-direction: column;
      gap: 1.2em;
    }

    &__label {
      font-family: var(--mono-font-family);
    }

    &__input-wrapper {
      column-gap: var(--default-gap);
    }

    :global(&__input-field) {
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

  .glyph {
    margin: var(--default-gap) 0;
    height: 1px;
  }

  .glyph:after {
    content: "↑↓";
    display: inline-block;
    position: relative;
    top: -1.2em;
    color: var(--divider-border-color);
    border: 1px solid var(--divider-border-color);
    border-radius: 2em;
    padding: 0.5em 1.25em;
    background-color: var(--divider-background-color);
  }
</style>
