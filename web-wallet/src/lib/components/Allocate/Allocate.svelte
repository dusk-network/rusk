<svelte:options immutable={true} />

<script>
  import { fade } from "svelte/transition";
  import { createEventDispatcher, onMount } from "svelte";
  import {
    mdiArrowUpBoldBoxOutline,
    mdiShieldLock,
    mdiShieldLockOpenOutline,
  } from "@mdi/js";
  import { areValidGasSettings, deductLuxFeeFrom } from "$lib/contracts";
  import { duskToLux, luxToDusk } from "$lib/dusk/currency";
  import { calculateAdaptiveCharCount, middleEllipsis } from "$lib/dusk/string";
  import { logo } from "$lib/dusk/icons";
  import {
    Badge,
    Icon,
    Textbox,
    Wizard,
    WizardStep,
  } from "$lib/dusk/components";
  import {
    AppAnchorButton,
    GasFee,
    GasSettings,
    OperationResult,
  } from "$lib/components";

  /** @type {(to: string, amount: number, gasPrice: number, gasLimit: number) => Promise<string>} */
  export let execute;

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

  /** @type {number} */
  export let shieldedBalance;

  /** @type {number} */
  export let unshieldedBalance;

  /** @type {boolean} */
  let isNextButtonDisabled = false;

  /** @type {boolean} */
  let isGasValid = false;

  let { gasLimit, gasPrice } = gasSettings;

  /** @type {number} */
  let shieldedAmount = shieldedBalance;

  /** @type {number} */
  let unshieldedAmount = unshieldedBalance;

  /** @type {number} */
  let screenWidth = window.innerWidth;

  const minAmount = 0.000000001;
  const dispatch = createEventDispatcher();
  const resetOperation = () => dispatch("operationChange", "");

  onMount(() => {
    isGasValid = areValidGasSettings(gasPrice, gasLimit);

    const resizeObserver = new ResizeObserver((entries) => {
      const entry = entries[0];

      screenWidth = entry.contentRect.width;
    });

    resizeObserver.observe(document.body);

    return () => resizeObserver.disconnect();
  });

  $: luxFee = gasLimit * gasPrice;
  $: fee = formatter(luxToDusk(luxFee));
  $: isFromUnshielded = shieldedAmount > shieldedBalance;
  $: isFromShielded = unshieldedAmount > unshieldedBalance;
  $: isNextButtonDisabled = !(isFromUnshielded || isFromShielded);
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
      <div in:fade|global class="operation__send">
        <p>
          Edit the value to change the allocation of your Dusk between your
          shielded or public account.
        </p>

        <fieldset class="operation__fieldset">
          <div class="operation__send-amount operation__label">Shielded</div>

          <div
            class="operation__send-amount operation__address operation__space-between"
          >
            <Icon path={mdiShieldLock} />
            {middleEllipsis(
              shieldedAddress,
              calculateAdaptiveCharCount(screenWidth, 320, 640, 5, 20)
            )}
          </div>

          <div class="operation__send-amount operation__input">
            <Textbox
              className="operation__input-field"
              bind:value={shieldedAmount}
              required
              type="number"
              min={minAmount}
              max={deductLuxFeeFrom(
                shieldedBalance + unshieldedBalance,
                luxFee
              )}
              step="0.000000001"
              on:input={() => {
                unshieldedAmount = +(
                  shieldedBalance +
                  unshieldedBalance -
                  shieldedAmount
                ).toFixed(9);
              }}
            />
            <Icon
              data-tooltip-id="main-tooltip"
              data-tooltip-text="DUSK"
              path={logo}
            />
          </div>

          <hr class="glyph" />

          <div class="operation__send-amount operation__label">Unshielded</div>

          <div
            class="operation__send-amount operation__address operation__space-between"
          >
            <Icon path={mdiShieldLockOpenOutline} />
            {middleEllipsis(
              unshieldedAddress,
              calculateAdaptiveCharCount(screenWidth, 320, 640, 5, 20)
            )}
          </div>

          <div class="operation__send-amount operation__input">
            <Textbox
              className="operation__input-field"
              bind:value={unshieldedAmount}
              required
              type="number"
              min={minAmount}
              max={deductLuxFeeFrom(
                unshieldedBalance + shieldedBalance,
                luxFee
              )}
              step="0.000000001"
              id="unshielded-amount"
              on:input={() => {
                shieldedAmount = +(
                  unshieldedBalance +
                  shieldedBalance -
                  unshieldedAmount
                ).toFixed(9);
              }}
            />
            <Icon
              data-tooltip-id="main-tooltip"
              data-tooltip-text="DUSK"
              path={logo}
            />
          </div>
        </fieldset>

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
      nextButton={{
        icon: { path: mdiArrowUpBoldBoxOutline, position: "before" },
        label: "SEND",
        variant: "primary",
      }}
    >
      <div in:fade|global class="operation__send">
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
              {isFromUnshielded
                ? `${formatter(luxToDusk(duskToLux(unshieldedBalance) - duskToLux(unshieldedAmount)))} DUSK`
                : `${formatter(luxToDusk(duskToLux(shieldedBalance) - duskToLux(shieldedAmount)))} DUSK`}
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
              path={isFromUnshielded ? mdiShieldLockOpenOutline : mdiShieldLock}
            />
            <span>From</span>
          </dt>
          <dd class="operation__review-address">
            <span>
              {isFromUnshielded ? unshieldedAddress : shieldedAddress}
            </span>
          </dd>
        </dl>
        <dl class="operation__review-transaction">
          <dt class="review-transaction__label">
            <Icon
              path={isFromShielded ? mdiShieldLockOpenOutline : mdiShieldLock}
            />
            <span>To</span>
          </dt>
          <dd class="operation__review-address">
            <span>
              {isFromShielded ? unshieldedAddress : shieldedAddress}
            </span>
          </dd>
        </dl>
        <GasFee {fee} />
      </div>
    </WizardStep>
    <WizardStep step={2} {key} showNavigation={false}>
      <OperationResult
        errorMessage="Transaction failed"
        onBeforeLeave={resetOperation}
        operation={execute(
          isFromShielded ? unshieldedAddress : shieldedAddress,
          isFromShielded ? unshieldedAmount : shieldedAmount,
          gasPrice,
          gasLimit
        )}
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

    &__address {
      font-family: var(--mono-font-family);
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

    &__send {
      display: flex;
      flex-direction: column;
      gap: 1.2em;
    }

    &__label {
      font-family: var(--mono-font-family);
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

  hr.glyph:after {
    content: "↑↓";
  }
</style>
