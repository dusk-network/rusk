<svelte:options immutable={true} />

<script>
  import { createEventDispatcher, onMount } from "svelte";
  import { mdiArrowLeft, mdiHistory } from "@mdi/js";
  import { parseUnits, parseEther } from "viem";
  import { switchChain, writeContract } from "@wagmi/core";

  import { Button, Icon, Textbox, Select } from "$lib/dusk/components";
  import { AppAnchor, AppAnchorButton } from "$lib/components";
  import { account, getAccountBalance, modal, wagmiConfig, walletDisconnect } from "$lib/web3/walletConnection";
  import { logo } from "$lib/dusk/icons";
  import { getDecimalSeparator } from "$lib/dusk/number";
  import {
    cleanNumberString
  } from "$lib/dusk/string";
  import { createCurrencyFormatter, duskToLux } from "$lib/dusk/currency";
  import { walletStore, settingsStore } from "$lib/stores";
  import { tokens } from "./tokenConfig";
  import { executeEvmBridgeDeposit } from "$lib/contracts";
  import wasmPath from "$lib/vendor/standard_bridge_dd_opt.wasm?url";

  /** @type {string} */
  export let bridgeNetwork;

  /** @type {string} */
  export let duskDsBalance = "0";

  const VITE_BRIDGE_CONTRACT_ID = import.meta.env.VITE_BRIDGE_CONTRACT_ID;
  const VITE_EVM_BRIDGE_CONTRACT_ADDRESS = import.meta.env.VITE_EVM_BRIDGE_CONTRACT_ADDRESS;

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
  const { language } = $settingsStore;

  $: ({ isConnected, address } = $account);
  $: ({ balance, currentProfile, profiles, syncStatus, useContract } = $walletStore);

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
      return duskFormatter(accountBalance.formatted);
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
    { value: "duskEvm", label: `DuskEVM (${connectedWalletBalance} DUSK)` },
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
  $: duskFormatter = createCurrencyFormatter(language, "DUSK", 9);
</script>
<article class="bridge">
  <header class="bridge__header">
    <h3 class="h4">Bridge</h3>
    <div class="bridge__header-icons">
      <AppAnchor href="/dashboard/bridge/transactions">
        <Icon path={mdiHistory} />
      </AppAnchor>
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

              function bytesToHex(arr) {
                let hex = "0x";
                for (let i = 0; i < arr.length; i++) {
                  const v = arr[i] & 0xff;
                  hex += v.toString(16).padStart(2, "0");
                }
                return hex;
              }

              // const utf8Encode = new TextEncoder();
              // const publicKey = currentProfile.account.toString();
              // console.log("Bridging from DuskEVM to DuskDS for account:", publicKey);
              // const extraDataBytes = Uint8Array.from([0, 0, 0, 0, 0, 0, 0, 96, 184, 161, 179, 178, 146, 226, 193, 40, 106, 178, 119, 149, 71, 229, 22, 220, 85, 240, 160, 222, 126, 15, 79, 90, 238, 228, 35, 225, 66, 75, 115, 22, 85, 191, 151, 47, 252, 140, 185, 41, 1, 137, 201, 26, 122, 202, 217, 205, 20, 55, 169, 162, 222, 142, 100, 161, 184, 160, 124, 188, 169, 38, 207, 19, 197, 48, 107, 28, 245, 74, 39, 17, 93, 145, 212, 247, 19, 34, 15, 118, 176, 87, 15, 129, 254, 215, 238, 220, 187, 177, 3, 113, 12, 14, 109, 39, 0, 0, 0, 0, 0, 0, 0, 193, 200, 235, 67, 154, 136, 10, 23, 196, 200, 144, 32, 2, 19, 169, 108, 233, 84, 14, 97, 43, 63, 170, 59, 240, 5, 23, 106, 46, 21, 46, 142, 226, 41, 72, 221, 62, 208, 104, 11, 13, 59, 244, 174, 71, 30, 112, 8, 7, 235, 120, 0, 0, 248, 222, 83, 25, 122, 239, 32, 182, 89, 196, 54, 114, 144, 39, 4, 5, 186, 83, 76, 50, 62, 225, 72, 239, 97, 5, 12, 110, 0, 131, 23, 40, 52, 3, 206, 92, 196, 205, 146, 17, 80, 62, 49, 20, 132, 2, 92, 54, 161, 213, 48, 109, 144, 93, 16, 2, 240, 170, 46, 246, 17, 28, 8, 81, 176, 40, 136, 178, 55, 111, 244, 209, 195, 79, 170, 142, 95, 27, 104, 188, 206, 161, 199, 101, 37, 175, 182, 54, 22, 229, 236, 13, 31, 198, 111, 7, 180, 101, 248, 32, 37, 48, 200, 205, 147, 141, 41, 171, 11, 4, 1, 186, 10, 5, 100, 137, 184, 138, 125, 1, 8, 1, 181, 126, 183, 209, 156, 53, 238, 94, 146, 234, 195, 157, 152, 169, 178, 72, 18, 25, 0]);    // extraData
              let args = []
              walletStore.useContract(VITE_BRIDGE_CONTRACT_ID, wasmPath).then((contract) => {
                console.log("contract loaded:", contract);
                args = [2_000_000, bytesToHex(contract.encode("encode_ds_address", currentProfile.account.toString()))];
                console.log({args});
              });
              
              await switchChain(wagmiConfig, { chainId: 310 })
              const hash = await writeContract(wagmiConfig, {
                address: VITE_EVM_BRIDGE_CONTRACT_ADDRESS,
                abi: ABI,
                functionName: "bridgeETH",
                args,
                value: parseEther(amount),
                chainId: 310, // DuskEVM chain ID
              });

              console.log("Withdrawal tx hash:", hash);

            } else if (originNetwork === "duskDs" && destinationNetwork === "duskEvm") {
              // Deposit from DuskDS to DuskEVM
              const hash = await executeEvmBridgeDeposit(address, duskToLux(amount), 1n, 2000000n); //FIXME: params

              console.log("Deposit tx hash:", hash);
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
