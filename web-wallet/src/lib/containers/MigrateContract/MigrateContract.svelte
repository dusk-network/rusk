<svelte:options immutable={true} />

<script>
  import { mdiArrowLeft, mdiArrowRight, mdiWalletOutline } from "@mdi/js";
  import { getAccount, switchChain } from "@wagmi/core";
  import { formatUnits, parseUnits } from "viem";
  import { onDestroy, onMount } from "svelte";
  import { tokens } from "./tokenConfig";
  import { getDecimalSeparator } from "$lib/dusk/number";
  import {
    calculateAdaptiveCharCount,
    cleanNumberString,
    middleEllipsis,
  } from "$lib/dusk/string";
  import {
    AppAnchor,
    AppAnchorButton,
    AppImage,
    ApproveMigration,
    ExecuteMigration,
  } from "$lib/components";
  import {
    Button,
    ExclusiveChoice,
    Icon,
    Stepper,
    Textbox,
  } from "$lib/dusk/components";
  import { logo } from "$lib/dusk/icons";
  import { settingsStore, walletStore } from "$lib/stores";
  import {
    account,
    modal,
    wagmiConfig,
    walletDisconnect,
  } from "$lib/migration/walletConnection";
  import { getBalanceOfCoin } from "$lib/migration/migration";

  /** @type {string} */
  export let migrationNetwork;

  const { darkMode } = $settingsStore;

  /**
   * We force the type here, because the Migrate Contract
   * won't be enabled if we're not on the networks below.
   * See `src/lib/contracts/contract-descriptors.js`.
   */
  const network = /** @type {"mainnet" | "testnet"} */ (
    migrationNetwork.toLowerCase()
  );

  const { ["ERC-20"]: erc20, ["BEP-20"]: bep20 } = tokens[network];

  const options = ["ERC-20", "BEP-20"];

  // The minimum allowed amount to be migrated expressed as a string
  const minAmount = "0.000000001";

  const ercDecimals = 18;

  const duskDecimals = 9;

  /** @type {TokenNames} */
  let selectedChain = erc20.name;

  /** @type {boolean} */
  const migrationInProgress = false;

  /** @type {bigint} */
  let connectedWalletBalance;

  /** @type {string} */
  let amount = "";

  /** @type {HTMLInputElement | null} */
  let amountInput;

  /** @type {boolean} */
  let isMigrationInitialized = false;

  /** @type {number} */
  let screenWidth = window.innerWidth;

  /** @type {number} */
  let migrationStep = 0;

  /** @type {boolean} */
  let isInputDisabled = false;

  $: ({ address, chainId, isConnected } = $account);
  $: ({ currentProfile } = $walletStore);
  $: currentAddress = currentProfile ? currentProfile.address.toString() : "";
  $: isAmountValid =
    !!amount &&
    parseUnits(amount.replace(",", "."), ercDecimals) >=
      parseUnits(minAmount, ercDecimals) &&
    parseUnits(amount.replace(",", "."), ercDecimals) <= connectedWalletBalance;
  $: amount = slashDecimals(cleanNumberString(amount, getDecimalSeparator()));

  /**
   *  Triggers the switchChain event and reverts the ExclusiveChoice UI selected option if an error is thrown
   *
   * @param {number} id - the chain id of the desired smart contract
   */
  async function handleSwitchChain(id) {
    try {
      await switchChain(wagmiConfig, { chainId: id });
      connectedWalletBalance = await getBalance();
    } catch (e) {
      selectedChain = chainId === erc20.chainId ? erc20.name : bep20.name;
    }
  }

  /** Emits the switchChain event to the third-party wallet when the ExclusiveChoice UI is interacted with  */
  // @ts-ignore
  async function onChainSwitch(e) {
    if (!isConnected) {
      return;
    }
    amount = "";
    const chainIdToSwitchTo =
      e.target?.value === bep20.name ? bep20.chainId : erc20.chainId;
    await handleSwitchChain(chainIdToSwitchTo);
  }

  /**
   * Checks if the chainId of the selected option of ExclusiveChoice UI is different than the chainId of the smart contract in the third-party wallet
   * and triggers an event to set the chainId if there is a difference
   */
  async function switchToSelectedChain() {
    const currentChainId =
      selectedChain === erc20.name ? erc20.chainId : bep20.chainId;
    if (chainId !== currentChainId) {
      await handleSwitchChain(currentChainId);
    }
  }

  async function getBalance() {
    const walletAccount = getAccount(wagmiConfig);

    if (!walletAccount.address) {
      throw new Error("Address is undefined");
    }

    return await getBalanceOfCoin(
      walletAccount.address,
      tokens[network][selectedChain].contract
    );
  }

  function incrementStep() {
    migrationStep++;
  }

  /**
   * @param {string} numberAsString
   * @returns {string}
   */
  function slashDecimals(numberAsString) {
    const separator = numberAsString.includes(".") ? "." : ",";
    const [integer, decimal] = numberAsString.split(separator);
    return decimal
      ? `${integer}${separator}${decimal.slice(0, duskDecimals)}`
      : numberAsString;
  }

  onMount(() => {
    const resizeObserver = new ResizeObserver((entries) => {
      const entry = entries[0];
      screenWidth = entry.contentRect.width;
    });

    (async () => {
      if (isConnected) {
        await walletDisconnect();
      }

      modal.subscribeEvents(async (e) => {
        if (e.data.event === "CONNECT_SUCCESS") {
          switchToSelectedChain();
          connectedWalletBalance = await getBalance();
        }
      });

      amountInput = document.querySelector(".migrate__input-field");

      resizeObserver.observe(document.body);
    })();

    return () => resizeObserver.disconnect();
  });

  onDestroy(async () => {
    await walletDisconnect();
  });
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
          width="37"
          height="27"
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
    <ExclusiveChoice
      {options}
      bind:value={selectedChain}
      on:change={onChainSwitch}
    />
    {#if isConnected && address && connectedWalletBalance}
      <p class="migrate__token-header">Connected Wallet:</p>
      <p class="migrate__token-address">
        {middleEllipsis(address, calculateAdaptiveCharCount(screenWidth))}
      </p>
      <div class="migrate__token-balance">
        Balance: <span
          >{slashDecimals(formatUnits(connectedWalletBalance, ercDecimals))}
          {selectedChain} DUSK</span
        >
      </div>
    {/if}
  </div>

  {#if isConnected && connectedWalletBalance}
    <div class="migrate__amount">
      <div class="migrate__amount-header">
        <div class="migrate__amount-token">
          {#if selectedChain === erc20.name}
            <AppImage
              src={darkMode ? "/eth_dusk_light.svg" : "/eth_dusk.svg"}
              alt="Ethereum Dusk"
              width="37"
              height="27"
            />
          {:else}
            <AppImage
              src={darkMode ? "/binance_dusk_light.svg" : "/binance_dusk.svg"}
              alt="Binance Dusk"
              width="37"
              height="27"
            />
          {/if}
          <p class="migrate__amount-currency">DUSK {selectedChain}</p>
        </div>

        <Button
          size="small"
          variant="tertiary"
          on:click={() => {
            if (amountInput) {
              amountInput.value = formatUnits(
                connectedWalletBalance,
                ercDecimals
              );
            }
            amount = slashDecimals(
              formatUnits(connectedWalletBalance, ercDecimals)
            );
          }}
          text="USE MAX"
          disabled={isInputDisabled}
        />
      </div>

      <Textbox
        className="migrate__input-field {!isAmountValid
          ? 'migrate__input-field--invalid'
          : ''}"
        bind:value={amount}
        required
        type="text"
        placeholder="Amount"
        disabled={isInputDisabled}
      />
    </div>
  {/if}

  {#if isConnected && isAmountValid && isMigrationInitialized}
    <div class="migrate__wizard">
      <Stepper steps={2} activeStep={migrationStep} variant="secondary" />

      {#if migrationStep === 0}
        <ApproveMigration
          on:incrementStep={incrementStep}
          on:initApproval={() => {
            isInputDisabled = true;
          }}
          on:errorApproval={() => {
            isInputDisabled = false;
          }}
          amount={parseUnits(amount.replace(",", "."), ercDecimals)}
          chainContract={tokens[network][selectedChain].contract}
          migrationContract={tokens[network][selectedChain].migrationContract}
        />
      {:else}
        <ExecuteMigration
          amount={parseUnits(amount.replace(",", "."), ercDecimals)}
          {currentAddress}
          migrationContract={tokens[network][selectedChain].migrationContract}
        />
      {/if}
    </div>
  {/if}

  {#if !isConnected}
    <Button
      icon={{ path: mdiWalletOutline }}
      text={`CONNECT TO  ${selectedChain === tokens[network]["ERC-20"].name ? "ETHEREUM" : "BSC"}`}
      on:click={() => {
        modal.open();
      }}
    />
  {:else if !isMigrationInitialized && address}
    <Button
      text="INITIALIZE MIGRATION"
      on:click={() => {
        isMigrationInitialized = true;
      }}
      disabled={!isAmountValid}
    />
  {/if}
</article>
<AppAnchorButton
  href="/dashboard"
  icon={{ path: mdiArrowLeft }}
  text="Back"
  variant="tertiary"
/>

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
      font-family: var(--mono-font-family);
      text-align: center;
    }

    &__token-balance {
      display: flex;
      justify-content: space-between;
      font-size: 0.875em;

      span {
        font-weight: 500;
        font-family: var(--mono-font-family);
      }
    }

    &__amount-header {
      display: flex;
      justify-content: space-between;
    }

    &__amount-currency {
      font-size: 0.875em;
    }
  }

  :global(.dusk-textbox.migrate__input-field) {
    background-color: var(--non-button-control-bg-color);
  }

  :global(.dusk-textbox.migrate__input-field--invalid) {
    color: var(--error-color);
  }
</style>
