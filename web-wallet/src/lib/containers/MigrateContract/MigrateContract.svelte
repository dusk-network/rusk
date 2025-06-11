<svelte:options immutable={true} />

<script>
  import {
    mdiArrowLeft,
    mdiArrowRight,
    mdiCheckDecagramOutline,
    mdiWalletOutline,
  } from "@mdi/js";
  import { switchChain } from "@wagmi/core";
  import { formatUnits, parseUnits } from "viem";
  import { onMount } from "svelte";
  import { tokens } from "./tokenConfig";
  import { getDecimalSeparator } from "$lib/dusk/number";
  import {
    calculateAdaptiveCharCount,
    cleanNumberString,
    middleEllipsis,
  } from "$lib/dusk/string";
  import {
    AppAnchorButton,
    AppImage,
    ApproveMigration,
    Banner,
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
  $: ({ isConnected, address } = $account);

  /**
   * We force the type here, because the Migrate Contract
   * won't be enabled if we're not on the networks below.
   * See `src/lib/contracts/contract-descriptors.js`.
   */
  const network = /** @type {"mainnet" | "testnet"} */ (
    migrationNetwork.toLowerCase()
  );

  const { ["ERC-20"]: erc20, ["BEP-20"]: bep20 } = tokens[network];

  $: options = [
    {
      disabled: isInputDisabled,
      label: "ERC-20",
      value: "ERC-20",
    },
    {
      disabled: isInputDisabled,
      label: "BEP-20",
      value: "BEP-20",
    },
  ];

  // The minimum allowed amount to be migrated expressed as a string
  const minAmount = "0.000000001";

  const ercDecimals = 18;

  const duskDecimals = 9;

  /** @type {TokenNames} */
  let selectedChain = erc20.name;

  /** @type {undefined | bigint} */
  let connectedWalletBalance;

  /** @type {string} */
  let amount = "";

  /** @type {boolean} */
  let isMigrationInitialized = false;

  /** @type {number} */
  let screenWidth = window.innerWidth;

  /** @type {number} */
  let migrationStep = 0;

  /** @type {boolean} */
  let isInputDisabled = false;

  const steps = [{ label: "Approve" }, { label: "Migrate" }, { label: "Done" }];

  $: ({ currentProfile } = $walletStore);
  $: moonlightAccount = currentProfile?.account.toString();

  $: isAmountValid = (() => {
    if (!amount) {
      return false;
    }
    try {
      const parsedAmount = parseUnits(amount.replace(",", "."), ercDecimals);
      const minParsed = parseUnits(minAmount, ercDecimals);
      return (
        parsedAmount >= minParsed &&
        parsedAmount <= (connectedWalletBalance ?? 0n)
      );
    } catch {
      return false;
    }
  })();

  $: amount = slashDecimals(cleanNumberString(amount, getDecimalSeparator()));

  /**
   *  Triggers the switchChain event and reverts the ExclusiveChoice UI selected option if an error is thrown
   *
   * @param {number} id - the chain id of the desired smart contract
   */
  async function handleSwitchChain(id) {
    try {
      if (isConnected) {
        await switchChain(wagmiConfig, { chainId: id });
        connectedWalletBalance = await getBalance();
      }
    } catch {
      selectedChain =
        $account.chainId === erc20.chainId ? erc20.name : bep20.name;
    }
  }

  /** Emits the switchChain event to the third-party wallet when the ExclusiveChoice UI is interacted with  */
  // @ts-ignore
  async function onChainSwitch(e) {
    if (!isConnected) {
      return;
    }
    amount = "";
    connectedWalletBalance = undefined;
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
    await handleSwitchChain(currentChainId);
  }

  async function getBalance() {
    try {
      if (!address) {
        throw new Error("Wallet not connected.");
      }
      return await getBalanceOfCoin(
        address,
        tokens[network][selectedChain].tokenContract
      );
    } catch {
      return 0n;
    }
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

    resizeObserver.observe(document.body);

    modal.subscribeEvents(handleModalEvents);

    // Workaround for when the user navigates back to the page
    // Just in case if beforeunload doesn't get triggered
    if ($account.isConnecting) {
      walletDisconnect();
    }

    // Workaround for when the user navigates
    // back to the page from the dashboard
    if ($account.isConnected) {
      switchToSelectedChain();
    }

    return () => {
      resizeObserver.disconnect();
    };
  });

  /**
   * Handles modal state changes.
   * @param {import('@reown/appkit').EventsControllerState} newEvent
   */
  const handleModalEvents = async (newEvent) => {
    // Check if the newState contains an event that matches "CONNECT_SUCCESS"
    if (newEvent.data.event === "CONNECT_SUCCESS") {
      await switchToSelectedChain();
    }
  };
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

  <div class="migrate__token">
    <p class="migrate__token-header">From:</p>
    <ExclusiveChoice
      {options}
      bind:value={selectedChain}
      on:change={onChainSwitch}
    />
    {#if isConnected && address}
      <p class="migrate__token-header">Connected Wallet:</p>
      <p class="migrate__token-address">
        {middleEllipsis(address, calculateAdaptiveCharCount(screenWidth))}
      </p>
      <span class="migrate__token-balance">
        Balance:
        {#if connectedWalletBalance === undefined}
          <span>Loading...</span>
        {:else}
          <span
            >{slashDecimals(
              formatUnits(connectedWalletBalance ?? 0n, ercDecimals)
            )}
            {selectedChain} DUSK</span
          >
        {/if}
      </span>
    {/if}
  </div>

  {#if isConnected && address && connectedWalletBalance && connectedWalletBalance > 0n}
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
            amount = formatUnits(connectedWalletBalance ?? 0n, ercDecimals);
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
      <Stepper {steps} activeStep={migrationStep} />
      {#if migrationStep === 0}
        <ApproveMigration
          on:incrementStep={() => migrationStep++}
          on:initApproval={() => {
            isInputDisabled = true;
          }}
          on:errorApproval={() => {
            isInputDisabled = false;
          }}
          amount={parseUnits(amount.replace(",", "."), ercDecimals)}
          chainContract={tokens[network][selectedChain].tokenContract}
          migrationContract={tokens[network][selectedChain].migrationContract}
        />
      {:else if migrationStep === 1}
        <ExecuteMigration
          on:incrementStep={() => migrationStep++}
          amount={parseUnits(amount.replace(",", "."), ercDecimals)}
          currentAddress={moonlightAccount ?? ""}
          migrationContract={tokens[network][selectedChain].migrationContract}
        />
      {:else}
        <div class="migrate__execute">
          <div class="migrate__execute-approval">
            <Icon path={mdiCheckDecagramOutline} size="large" />
            <p>Migration request accepted!</p>
          </div>
          <Banner title="Migration Request Accepted" variant="info">
            <p>
              The migration request has now been accepted on chain. We will
              process the request in the background. Receiving your DUSK might
              take some time. Check your wallet balance again later.
            </p>
          </Banner>
        </div>
      {/if}
    </div>
  {/if}

  {#if !isConnected}
    <Button
      icon={{ path: mdiWalletOutline }}
      text={`CONNECT TO ${selectedChain === tokens[network]["ERC-20"].name ? "ETHEREUM" : "BSC"}`}
      on:click={() => modal.open()}
    />
  {:else if connectedWalletBalance === 0n}
    <Banner variant="warning" title="No DUSK available">
      <p>The connected wallet has no DUSK tokens available.</p>
    </Banner>
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

{#if isConnected}
  <Button
    text="Manage Wallet"
    on:click={() => {
      modal.open();
    }}
    variant="tertiary"
  />
{/if}

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

    &__wizard {
      margin-top: var(--default-gap);
      gap: 1.25em;
      display: flex;
      flex-direction: column;
    }

    &__execute {
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: var(--default-gap);

      &-approval {
        display: flex;
        flex-direction: column;
        align-items: center;
        gap: var(--default-gap);
      }
    }

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
