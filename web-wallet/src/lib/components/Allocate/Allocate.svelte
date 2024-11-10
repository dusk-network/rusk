<svelte:options immutable={true} />

<script>
  import { fade } from "svelte/transition";
  import { onMount } from "svelte";
  import {
    mdiArrowUpBoldBoxOutline,
    mdiShieldLock,
    mdiShieldLockOpenOutline,
  } from "@mdi/js";
  import { areValidGasSettings, deductLuxFeeFrom } from "$lib/contracts";
  import { luxToDusk } from "$lib/dusk/currency";
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
  import BigIntInput from "../BigIntInput/BigIntInput.svelte";

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

  /** @type {boolean} */
  let isNextButtonDisabled = false;

  /** @type {boolean} */
  let isGasValid = false;

  let { gasLimit, gasPrice } = gasSettings;

  /** @type {bigint} */
  let shieldedAmount = shieldedBalance;

  /** @type {bigint} */
  let unshieldedAmount = unshieldedBalance;

  /** @type {number} */
  let screenWidth = window.innerWidth;

  const minAmount = 0.000000001;

  onMount(() => {
    isGasValid = areValidGasSettings(gasPrice, gasLimit);

    const resizeObserver = new ResizeObserver((entries) => {
      const entry = entries[0];

      screenWidth = entry.contentRect.width;
    });

    resizeObserver.observe(document.body);

    return () => resizeObserver.disconnect();
  });

  $: fee = gasLimit * gasPrice;
  $: balanceStatus = {
    isFromUnshielded: shieldedAmount > shieldedBalance,
    isFromShielded: unshieldedAmount > unshieldedBalance,
  };

  $: isNextButtonDisabled = !(
    balanceStatus.isFromShielded || balanceStatus.isFromUnshielded
  );
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
          shielded or public account.
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
            <BigIntInput
              className="operation__input-field"
              id="shielded-amount"
              bind:value={shieldedAmount}
              min={minAmount}
              max={deductLuxFeeFrom(
                luxToDusk(shieldedBalance + unshieldedBalance),
                fee
              )}
              on:input={() => {
                unshieldedAmount =
                  shieldedBalance + unshieldedBalance - shieldedAmount;
              }}
            />
            <Icon
              data-tooltip-id="main-tooltip"
              data-tooltip-text="DUSK"
              path={logo}
            />
          </div>

          <hr class="glyph" />

          <p class="operation__label">Unshielded</p>

          <div class="operation__address-wrapper">
            <Icon path={mdiShieldLockOpenOutline} />
            {middleEllipsis(
              unshieldedAddress,
              calculateAdaptiveCharCount(screenWidth, 320, 640, 5, 20)
            )}
          </div>

          <div class="operation__input-wrapper">
            <BigIntInput
              className="operation__input-field"
              bind:value={unshieldedAmount}
              min={minAmount}
              max={deductLuxFeeFrom(
                luxToDusk(unshieldedBalance + shieldedBalance),
                fee
              )}
              on:input={() => {
                shieldedAmount =
                  unshieldedBalance + shieldedBalance - unshieldedAmount;
              }}
              id="unshielded-amount"
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
              {balanceStatus.isFromUnshielded
                ? `${formatter(luxToDusk(unshieldedBalance - unshieldedAmount))} DUSK`
                : `${formatter(luxToDusk(shieldedBalance - shieldedAmount))} DUSK`}
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
              path={balanceStatus.isFromUnshielded
                ? mdiShieldLockOpenOutline
                : mdiShieldLock}
            />
            <span>From</span>
          </dt>
          <dd class="operation__review-address">
            <span>
              {balanceStatus.isFromUnshielded
                ? unshieldedAddress
                : shieldedAddress}
            </span>
          </dd>
        </dl>
        <dl class="operation__review-transaction">
          <dt class="review-transaction__label">
            <Icon
              path={balanceStatus.isFromShielded
                ? mdiShieldLockOpenOutline
                : mdiShieldLock}
            />
            <span>To</span>
          </dt>
          <dd class="operation__review-address">
            <span>
              {balanceStatus.isFromShielded
                ? unshieldedAddress
                : shieldedAddress}
            </span>
          </dd>
        </dl>
        <GasFee {formatter} {fee} />
      </div>
    </WizardStep>
    <WizardStep step={2} {key} showNavigation={false}>
      <OperationResult
        errorMessage="Transaction failed"
        operation={balanceStatus.isFromUnshielded
          ? walletStore.shield(shieldedAmount, new Gas({ gasLimit, gasPrice }))
          : walletStore.unshield(
              unshieldedAmount,
              new Gas({ gasLimit, gasPrice })
            )}
        pendingMessage="Processing transaction"
        successMessage="Transaction completed"
      >
        <svelte:fragment slot="success-content" let:result={hash}>
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
