
<svelte:options immutable={true} />

<script>
  import { onDestroy, onMount } from "svelte";
  import { getBalance } from "@wagmi/core";

  import {
    account,
    modal,
    walletDisconnect,
  } from "$lib/web3/walletConnection";

  import { BridgeContract } from "$lib/containers";
  import { networkStore, operationsStore, walletStore, settingsStore } from "$lib/stores";
  import { createCurrencyFormatter, luxToDusk } from "$lib/dusk/currency";

  let connectedWalletBalance = 0;

  /** @param {string} id */
  function updateOperation(id) {
    operationsStore.update((store) => ({
      ...store,
      currentOperation: id,
    }));
  }

  onDestroy(() => {
    updateOperation("");
  });

  const { networkName } = $networkStore;
  const { language } = $settingsStore;

  $: ({ balance } = $walletStore);
  $: duskFormatter = createCurrencyFormatter(language, "DUSK", 9);
</script>

{#if ["mainnet", "testnet", "devnet", "localnet"].includes(networkName)}
  <BridgeContract
    bridgeNetwork={networkName}
    duskDsBalance={duskFormatter(luxToDusk(balance.unshielded.value))}
  />
{:else}
  <p>The bridge is not available on the current network.</p>
{/if}
