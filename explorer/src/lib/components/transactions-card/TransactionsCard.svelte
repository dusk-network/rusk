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
  export let txs;

  /** @type {Error | null}*/
  export let error;

  /** @type {Boolean} */
  export let loading;

  /** @type {number} */
  let clientWidth;

  $: classes = makeClassName(["transactions-card", className]);
</script>

<svelte:window bind:outerWidth={clientWidth} />
<DataCard
  on:retry
  data={txs}
  {error}
  {loading}
  className={classes}
  title="Transactions"
  button={{ action: () => goto("/transactions"), label: "All Transactions" }}
>
  {#if clientWidth > 768}
    <TransactionsTable data={txs} />
  {:else}
    {#each txs as tx (tx)}
      <TransactionsList data={tx} />
    {/each}
  {/if}
</DataCard>
