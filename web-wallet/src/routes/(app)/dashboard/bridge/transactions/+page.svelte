<svelte:options immutable={true} />

<script>
  import { EvmTransactions } from "$lib/components";
  import { settingsStore, walletStore } from "$lib/stores";
  import wasmPath from "$lib/vendor/standard_bridge_dd_opt.wasm?url";

  /** @type {import('./$types').PageData} */
  export let data;

  // /** @type {array | undefined} */
  // let items;

  // data.transactions.then((transactions) => {
  //   items = transactions;
  //   console.log("Bridge transactions:", items);
  // });

  const VITE_BRIDGE_CONTRACT_ID = import.meta.env.VITE_BRIDGE_CONTRACT_ID;
  const { language } = $settingsStore;

  $: ({ syncStatus } = $walletStore);

  
</script>

<div class="transactions">
  <h2 class="sr-only">Transactions</h2>
  <!-- <EvmTransactions
    items={walletStore.getEvmTransactions(VITE_BRIDGE_CONTRACT_ID, wasmPath)}
    {language}
    isSyncing={syncStatus.isInProgress}
    syncError={syncStatus.error}
  /> -->
  <EvmTransactions
    items={data.transactions}
    {language}
    isSyncing={syncStatus.isInProgress}
    syncError={syncStatus.error}
  />
</div>

<style lang="postcss">
  .transactions {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 1.375rem;
    overflow-y: auto;
    flex: 1;
  }
</style>
