<svelte:options immutable={true} />

<script>
  import { onDestroy } from "svelte";
  import { MigrateContract } from "$lib/containers";
  import { networkStore, operationsStore } from "$lib/stores";
  import { walletDisconnect } from "$lib/migration/walletConnection";

  /** @param {string} id */
  function updateOperation(id) {
    operationsStore.update((store) => ({
      ...store,
      currentOperation: id,
    }));
  }

  onDestroy(() => {
    walletDisconnect();
    updateOperation("");
  });

  const { networkName } = $networkStore;
</script>

{#if ["mainnet", "testnet"].includes(networkName)}
  <MigrateContract migrationNetwork={networkName} />
{/if}
