<svelte:options immutable={true} />

<script>
  import { EvmTransactions } from "$lib/components";
  import { settingsStore } from "$lib/stores";

  /** @type {import('./$types').PageData} */
  export let data;

  const { language } = $settingsStore;
</script>

<div class="transactions">
  <h2 class="sr-only">Transactions</h2>
  <EvmTransactions items={data.pendingWithdrawals} {language} />
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
