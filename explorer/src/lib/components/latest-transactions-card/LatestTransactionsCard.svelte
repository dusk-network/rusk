<svelte:options immutable={true} />

<script>
  import {
    DataCard,
    TransactionsList,
    TransactionsTable,
  } from "$lib/components";
  import { makeClassName } from "$lib/dusk/string";
  import { goto } from "$lib/navigation";

  /** @type {string | Undefined} */
  export let className = undefined;

  /** @type {Transaction[]}*/
  export let txns;

  /** @type {Error | null}*/
  export let error;

  /** @type {Boolean} */
  export let loading;

  /** @type {Boolean} */
  export let isOnHomeScreen = true;

  /** @type {Boolean} */
  export let displayTooltips = false;

  /** @type {AppStore} */
  export let appStore;

  $: classes = makeClassName(["latest-transactions-card", className]);
</script>

<DataCard
  on:retry
  data={txns}
  {error}
  {loading}
  className={classes}
  title="Transactions"
  headerButtonDetails={isOnHomeScreen
    ? {
        action: () => goto("/transactions"),
        disabled: false,
        label: "All Transactions",
      }
    : undefined}
>
  {#if $appStore.isSmallScreen}
    {#each txns as txn (txn)}
      <TransactionsList
        data={txn}
        mode={isOnHomeScreen ? "compact" : "full"}
        {displayTooltips}
      />
    {/each}
  {:else}
    <TransactionsTable
      data={txns}
      mode={isOnHomeScreen ? "compact" : "full"}
    />
  {/if}
</DataCard>
