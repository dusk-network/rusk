<svelte:options immutable={true} />

<script>
  import { onDestroy } from "svelte";

  import { updateOperation } from "$lib/contracts";
  import { BridgeContract } from "$lib/containers";
  import { networkStore } from "$lib/stores";

  const { networkName } = $networkStore;

  onDestroy(() => {
    updateOperation("");
  });
</script>

{#if import.meta.env.VITE_FEATURE_BRIDGE || false}
  {#if ["mainnet", "testnet", "devnet", "localnet"].includes(networkName)}
    <BridgeContract
      on:operationChange={({ detail }) => updateOperation(detail)}
    />
  {/if}
{/if}
