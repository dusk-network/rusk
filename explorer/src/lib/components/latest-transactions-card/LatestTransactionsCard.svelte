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

  /** @type {number} */
  let clientWidth;

  /** @type {Boolean} */
  export let isOnHomeScreen = true;

  $: classes = makeClassName(["latest-transactions-card", className]);
</script>

<svelte:window bind:outerWidth={clientWidth} />
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
        variant: "secondary",
      }
    : undefined}
>
  {#if clientWidth > 768}
    <TransactionsTable data={txns} />
  {:else}
    {#each txns as txn (txn)}
      <TransactionsList data={txn} />
    {/each}
  {/if}
</DataCard>
