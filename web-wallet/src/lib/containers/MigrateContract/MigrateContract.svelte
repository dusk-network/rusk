<svelte:options immutable={true} />

<script>
  import { AppAnchor, AppImage } from "$lib/components";
  import {
    Button,
    ExclusiveChoice,
    Icon,
    Textbox,
    Wizard,
    WizardStep,
  } from "$lib/dusk/components";
  import {
    mdiArrowRight,
    mdiCheckDecagramOutline,
    mdiInformationOutline,
    mdiTimerSand,
    mdiWalletOutline,
  } from "@mdi/js";
  import { createCurrencyFormatter } from "$lib/dusk/currency";
  import { logo } from "$lib/dusk/icons";
  import { calculateAdaptiveCharCount, middleEllipsis } from "$lib/dusk/string";
  import { onMount } from "svelte";
  import { settingsStore } from "$lib/stores";

  const ERC_TOKEN = "ERC-20";
  const BEP_TOKEN = "BEP-20";

  const options = [
    { disabled: false, label: ERC_TOKEN, value: ERC_TOKEN },
    { disabled: false, label: BEP_TOKEN, value: BEP_TOKEN },
  ];

  /** @type {String} */
  let selected = ERC_TOKEN;

  /** @type {Boolean} */
  const migrationInProgress = false;

  /** @type {Boolean} */
  let isWalletConnected = false;

  /** @type {String} */
  let connectedWalletAddress =
    "6rmam3FisEHih84oJMEsmi2GGVetDPmcjw7H4heeVbm79DwZUPvPJTBeFriLx85Cy2Q2cGisz32BeSYX99kNXjD";

  /** @type {Number} */
  let connectedWalletBalance = 10;

  /** @type {Number | undefined} */
  let amount;

  /** @type {HTMLInputElement | null} */
  let amountInput;

  /** @type {Boolean} */
  let isMigrationInitialized = false;

  /** @type {Number} */
  const gasFee = 1;

  /** @type {Boolean} */
  let isMigrationBeingApproved = false;

  /** @type {String}*/
  const estimatedTime = "45 min";

  /** @type {number} */
  let screenWidth = window.innerWidth;

  const { darkMode } = $settingsStore;

  const minAmount = 0.000000001;

  $: maxSpendable = connectedWalletBalance - gasFee;
  $: isAmountValid =
    typeof amount === "number"
      ? amount >= minAmount && amount <= maxSpendable
      : false;
  $: ({ language } = $settingsStore);
  $: duskFormatter = createCurrencyFormatter(language, "DUSK", 9);
  $: if (selected) {
    resetMigrationIfConnected();
  }

  onMount(() => {
    amountInput = document.querySelector(".migrate__input-field");

    const resizeObserver = new ResizeObserver((entries) => {
      const entry = entries[0];

      screenWidth = entry.contentRect.width;
    });

    resizeObserver.observe(document.body);

    return () => resizeObserver.disconnect();
  });

  /**
   * @param {Number} ms
   */
  function delay(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  function resetMigrationIfConnected() {
    if (isWalletConnected) {
      isWalletConnected = false;
      connectedWalletAddress = "";
      connectedWalletBalance = 10;
      amount = undefined;
      isMigrationInitialized = false;
      isMigrationBeingApproved = false;
    }
  }
</script>

<article class="migrate">
  <header class="migrate__header">
    <h3 class="h4">Migrate</h3>
    <div class="migrate__header-icons">
      <div>
        <AppImage
          src={darkMode ? "/binance_dusk_light.svg" : "/binance_dusk.svg"}
          alt="Binance Dusk"
          width="37"
          height="27"
        />
        <AppImage
          src={darkMode ? "/eth_dusk_light.svg" : "/eth_dusk.svg"}
          alt="Ethereum Dusk"
          width="20"
          height="25"
        />
      </div>
      <Icon path={mdiArrowRight} />
      <Icon path={logo} />
    </div>
  </header>

  {#if migrationInProgress}
    <div class="migrate__progress-notice">
      <span
        >Another migration is in progress. You can check the status <AppAnchor
          href="#">here</AppAnchor
        >.</span
      >
    </div>
  {/if}

  <div class="migrate__token">
    <p class="migrate__token-header">From:</p>
    <ExclusiveChoice {options} bind:value={selected} />
    {#if isWalletConnected}
      <p class="migrate__token-header">Connected Wallet:</p>
      <p class="migrate__token-address">
        {middleEllipsis(
          connectedWalletAddress,
          calculateAdaptiveCharCount(screenWidth)
        )}
      </p>
      <div class="migrate__token-balance">
        Balance: <span
          >{duskFormatter(connectedWalletBalance)}
          {selected} DUSK</span
        >
      </div>
    {/if}
  </div>

  {#if isWalletConnected}
    <div class="migrate__amount">
      <div class="migrate__amount-header">
        <div class="migrate__amount-token">
          {#if selected === ERC_TOKEN}
            <AppImage
              src={darkMode ? "/eth_dusk_light.svg" : "/eth_dusk.svg"}
              alt="Ethereum Dusk"
              width="20"
              height="25"
            />
          {:else}
            <AppImage
              src={darkMode ? "/binance_dusk_light.svg" : "/binance_dusk.svg"}
              alt="Binance Dusk"
              width="37"
              height="27"
            />
          {/if}
          <p class="migrate__amount-currency">DUSK {selected}</p>
        </div>

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

      <Textbox
        className="migrate__input-field"
        bind:value={amount}
        required
        type="number"
        min={minAmount}
        max={maxSpendable}
        step="0.000000001"
        placeholder="Amount"
      />
    </div>
  {/if}

  {#if isWalletConnected && !isAmountValid && typeof amount === "number"}
    <div class="migrate__amount-notice">Not enough balance</div>
  {/if}

  {#if isWalletConnected && isAmountValid && isMigrationInitialized}
    <div class="migrate__information">
      <div class="migrate__information-header">
        <p class="migrate__information-time">
          <span>
            Est. Time<Icon
              path={mdiInformationOutline}
              data-tooltip-id="main-tooltip"
              data-tooltip-text="Estimated time of migration"
            />
          </span>
          {estimatedTime}
        </p>
        <p class="migrate__information-fee">
          <span>
            Total Gas Fee<Icon
              path={mdiInformationOutline}
              data-tooltip-id="main-tooltip"
              data-tooltip-text="Total cost of gas"
            />
          </span>
          {gasFee}
        </p>
      </div>

      <Wizard steps={3} let:key>
        <WizardStep
          step={0}
          {key}
          showStepper={true}
          hideBackButton={true}
          nextButton={{
            action: async () => {
              isMigrationBeingApproved = true;
              await delay(10000);
            },
            disabled: isMigrationBeingApproved,
            icon: null,
            label: "APPROVE MIGRATION",
            variant: "primary",
          }}
        >
          {#if !isMigrationBeingApproved}
            <div class="migrate__information-notice">
              <p>DUSK token migration requires two transactions:</p>
              <ol class="migrate__information-list">
                <li>
                  Approve: Authorize the migration contract to spend your DUSK
                  tokens.
                </li>
                <li>
                  Migrate: Transfer your DUSK tokens to the migration contract.
                </li>
              </ol>
              <p>
                Both steps must be completed for a successful migration.<br
                /><br />Warning: Make sure your wallet has enough funds to pay
                for the gas.
              </p>
            </div>
          {:else}
            <div class="migrate__information-approval">
              <Icon path={mdiTimerSand} />
              <span>Approval in progress</span>
            </div>
          {/if}
        </WizardStep>
        <WizardStep
          step={1}
          {key}
          hideBackButton={true}
          showStepper={true}
          nextButton={{
            action: async () => {},
            icon: null,
            label: "EXECUTE MIGRATION",
            variant: "primary",
          }}
        >
          <div class="migrate__information-approval">
            <Icon path={mdiCheckDecagramOutline} />
            <span>Migration Approved</span>
          </div>
        </WizardStep>
        <WizardStep step={2} {key} showStepper={true} showNavigation={false}>
          <div class="migrate__information-approval">
            <Icon path={mdiCheckDecagramOutline} />
            <span>Migration in progress</span>
          </div>
          <div class="migrate__information-notice">
            <span
              >Migration takes some minutes to complete. Your transaction is
              being executed and you can check it <AppAnchor href="#"
                >here</AppAnchor
              >.</span
            >
          </div>
        </WizardStep>
      </Wizard>
    </div>
  {/if}

  {#if !isWalletConnected}
    <Button
      icon={{ path: mdiWalletOutline }}
      text={`CONNECT TO  ${selected === ERC_TOKEN ? "ETHEREUM" : "BSC"}`}
      on:click={() => {
        isWalletConnected = true;
      }}
    />
  {:else if !isMigrationInitialized}
    <Button
      text="INITIALIZE MIGRATION"
      on:click={() => {
        isMigrationInitialized = true;
      }}
      disabled={!isAmountValid}
    />
  {/if}
</article>

<style lang="postcss">
  .migrate {
    border-radius: 1.25em;
    background: var(--surface-color);
    display: flex;
    flex-direction: column;
    gap: var(--default-gap);
    padding: 1.25em;

    &__header {
      display: flex;
      justify-content: space-between;
    }

    &__header-icons {
      display: flex;
      align-items: center;
      gap: 0.675em;
    }

    &__token,
    &__amount {
      display: flex;
      flex-direction: column;
      gap: var(--default-gap);
      border-radius: 1.5em;
      padding: 0.75em;
      background-color: var(--background-color);
    }

    &__amount-token {
      display: flex;
      gap: var(--small-gap);
      align-items: center;
    }

    &__progress-notice {
      padding: 1em 1.375em;
      border-radius: 1.5em;
      border: 1px solid var(--primary-color);
    }

    &__token-header {
      font-weight: 500;
    }

    &__token-address {
      font-size: 1em;
      font-weight: 500;
      font-family: "Soehne Mono";
      text-align: center;
    }

    &__token-balance {
      display: flex;
      justify-content: space-between;
      font-size: 0.875em;

      span {
        font-weight: 500;
        font-family: "Soehne Mono";
      }
    }

    &__amount-header {
      display: flex;
      justify-content: space-between;
    }

    &__amount-currency {
      font-size: 0.875em;
    }

    &__amount-notice {
      padding: 1em 1.375em;
      border-radius: 0.675em;
      border: 1px solid var(--error-color);
      color: var(--error-color);
    }

    &__information-notice {
      font-size: 0.875em;
      line-height: 1.3125em;
      padding: 1em 1.375em;
      border-radius: 0.675em;
      border: 1px solid var(--primary-color);
      margin-top: 1.875em;
    }

    &__information-header {
      display: flex;
      justify-content: space-between;
      padding-bottom: 1.25em;
    }

    &__information-time,
    &__information-fee {
      display: flex;
      font-size: 0.875em;
      align-items: center;
      gap: var(--small-gap);

      span {
        display: flex;
        align-items: center;
      }
    }

    &__information-list {
      padding-left: 1.375em;
    }

    &__information-approval {
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: var(--default-gap);
      padding: 2.25em 0;
    }
  }

  :global(.dusk-textbox.migrate__input-field) {
    background-color: var(--non-button-control-bg-color);
  }
</style>
