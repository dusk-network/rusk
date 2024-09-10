<svelte:options immutable={true} />

<script>
  import { Transactions } from "$lib/components";
  import { settingsStore, walletStore } from "$lib/stores";

  const { language } = $settingsStore;

  $: ({ syncStatus } = $walletStore);
</script>

<div class="transactions">
  <h2 class="sr-only">Transactions</h2>

  <Transactions
    items={walletStore.getTransactionsHistory()}
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
