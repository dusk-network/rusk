<svelte:options immutable={true} />

<script>
  import { createEventDispatcher, onMount } from "svelte";
  import { mdiArrowLeft, mdiHistory } from "@mdi/js";
  import { parseUnits, parseEther } from "viem";
  import { switchChain, writeContract } from "@wagmi/core";

  import { Button, Icon, Textbox, Select } from "$lib/dusk/components";
  import { AppAnchorButton } from "$lib/components";
  import { account, getAccountBalance, modal, wagmiConfig, walletDisconnect } from "$lib/web3/walletConnection";
  import { logo } from "$lib/dusk/icons";
  import { getDecimalSeparator } from "$lib/dusk/number";
  import {
    cleanNumberString
  } from "$lib/dusk/string";
  import { duskToLux } from "$lib/dusk/currency";
  import { walletStore } from "$lib/stores";
  import { tokens } from "./tokenConfig";
  import { executeEvmBridgeDeposit } from "$lib/contracts";

  /** @type {string} */
  export let bridgeNetwork;

  /** @type {string} */
  export let duskDsBalance = "0";

  /** @type {number} */
  let screenWidth = window.innerWidth;

  /** @type {object} */
  let duskEvmBalance = null;

  /** @type {undefined | bigint} */
  let connectedWalletBalance;

  /** @type {string} */
  let amount = "";

  const dispatch = createEventDispatcher();
  const network = /** @type {"mainnet" | "testnet" | "devnet"} */ (
    bridgeNetwork.toLowerCase()
  );

  $: ({ isConnected, address } = $account);
  $: ({ balance, currentProfile, profiles, syncStatus } = $walletStore);

  onMount(async () => {
    const resizeObserver = new ResizeObserver((entries) => {
      const entry = entries[0];
      screenWidth = entry.contentRect.width;
    });

    resizeObserver.observe(document.body);

    if ($account.isConnecting) {
      await walletDisconnect();
    }

    return () => {
      resizeObserver.disconnect();
    };
  });

  /**
   * @param {string} numberAsString
   * @returns {string}
   */
  function slashDecimals(numberAsString) {
    const DUSK_DECIMALS = 9;
    const separator = numberAsString.includes(".") ? "." : ",";
    const [integer, decimal] = numberAsString.split(separator);
    return decimal
      ? `${integer}${separator}${decimal.slice(0, DUSK_DECIMALS)}`
      : numberAsString;
  }

  async function getBalance(accountAddress) {
    try {
      if (!accountAddress) {
        throw new Error("Wallet not connected.");
      }
      const accountBalance = await getAccountBalance(accountAddress);
      return accountBalance.value;
    } catch {
      return 0n;
    }
  }

  $: if (isConnected && address) {
    (async () => {
      connectedWalletBalance = await getBalance(address);
    })();
  }

  let baseOptions = [];

  $: baseOptions = [
    { value: "duskDs",  label: `DuskDS (${duskDsBalance} DUSK)` },
    { value: "duskEvm", label: `DuskEVM (${connectedWalletBalance ?? "…"} DUSK)` },
  ];
  $: originNetworkOptions = baseOptions.map((network) => ({
    ...network,
  }));
  $: originNetwork = baseOptions[0].value; // DuskDS
  $: destinationNetworkOptions = baseOptions.map((network) => ({
    ...network,
    disabled: network.value === originNetwork,
  }));
  $: destinationNetwork = destinationNetworkOptions.find(
    (option) => !option.disabled
  ).value;
  $: ({ isConnected, address } = $account);
  $: ({ balance, currentProfile, profiles, syncStatus } = $walletStore);
  $: amount = slashDecimals(cleanNumberString(amount, getDecimalSeparator()));
</script>
<article class="bridge">
  <header class="bridge__header">
    <h3 class="h4">Bridge</h3>
    <div class="bridge__header-icons">
      <Icon path={mdiHistory} />
    </div>
  </header>

  <div class="operation">
    {#if !isConnected}
      <Button text="CONNECT WALLET" on:click={() => modal.open()} />
    {:else}
        <fieldset class="operation__fieldset">
          <p class="operation__label">From</p>

          <div class="operation__input-wrapper">
            <Select
              bind:value={originNetwork}
              name="origin-network"
              options={originNetworkOptions}
            />
          </div>

          <!-- <div class="operation__input-wrapper">
            <Textbox
              className="operation__input-field"
              value={0}
              required
              type="number"
              max={1000}
              min="0"
              step="0.000000001"
              id="origin-amount"
              name="origin-amount"
            />
            <Icon
              data-tooltip-id="main-tooltip"
              data-tooltip-text="DUSK"
              path={logo}
            />
          </div> -->

          <hr class="glyph" />

          <p class="operation__label">To</p>

          <div class="operation__input-wrapper">
            <Select
              bind:value={destinationNetwork}
              name="destination-network"
              options={destinationNetworkOptions}
            />
          </div>

          <div class="operation__input-wrapper">
            <Textbox
              className="operation__input-field"
              required
              type="text"
              id="destination-amount"
              name="destination-amount"
              bind:value={amount}
            />
            <Icon
              data-tooltip-id="main-tooltip"
              data-tooltip-text="DUSK"
              path={logo}
            />
          </div>
        </fieldset>
        <Button
          className="operation__operation-button"
          text="Bridge Funds"
          on:click={async () => {
            console.log("origin:", originNetwork);
            console.log("destination:", destinationNetwork);

            if (originNetwork === "duskEvm" && destinationNetwork === "duskDs") {
              // Withdrawal from DuskEVM to DuskDS 

              const L2_BRIDGE_ADDRESS =
                "0x4200000000000000000000000000000000000010";
              const ABI = [
                {
                  type: "function",
                  name: "bridgeETH",
                  stateMutability: "payable",
                  inputs: [
                    { name: "minGasLimit", type: "uint32" },
                    { name: "extraData", type: "bytes" },
                  ],
                  outputs: [],
                },
              ];
              await writeContract(wagmiConfig, {
                address: L2_BRIDGE_ADDRESS,
                abi: ABI,
                functionName: "bridgeETH",
                args: [],
                value: parseEther("0.5"),
              });

            } else if (originNetwork === "duskDs" && destinationNetwork === "duskEvm") {
              // Deposit from DuskDS to DuskEVM
              await executeEvmBridgeDeposit(duskToLux(amount), 1n, 500n); //FIXME: params
            }
          }}
        />
    {/if}
    <AppAnchorButton
      className="operation__operation-button"
      href="/dashboard"
      icon={{ path: mdiArrowLeft }}
      on:click={() => {
        dispatch("operationChange", "");
      }}
      text="Back"
      variant="tertiary"
    />
  </div>

</article>

<style lang="postcss">
  .bridge {
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
  }

  .operation {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;

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

    &__input-wrapper {
      column-gap: var(--default-gap);
      display: flex;
      align-items: center;
      width: 100%;
    }

    & > :global(.operation__operation-button) {
      width: 100%;
      text-align: left;
    }
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
