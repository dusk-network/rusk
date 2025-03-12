<script>
  import { onMount } from "svelte";
  import { navigating, page } from "$app/stores";
  import { Card } from "$lib/dusk/components";
  import { AccountOverview, TransactionsCard } from "$lib/components/";
  import { duskAPI } from "$lib/services";
  import { appStore } from "$lib/stores";
  import { createDataStore } from "$lib/dusk/svelte-stores";

  const key = $page.url.searchParams.get("key");
  const dataStore = createDataStore(duskAPI.getMoonlightAccountTransactions);
  const getTransactions = () => {
    if (key) {
      dataStore.getData(key);
    }
  };

  onMount(getTransactions);

  $: if (
    $navigating &&
    $navigating.from?.route.id === $navigating.to?.route.id
  ) {
    $navigating.complete.then(getTransactions);
  }

  $: ({ data, error, isLoading } = $dataStore);
  $: ({ isSmallScreen } = $appStore);
</script>

{#if key}
  <section>
    <article>
      <AccountOverview accountAddress={key} />
    </article>

    <TransactionsCard
      on:retry={getTransactions}
      txns={data}
      {error}
      loading={isLoading}
      {isSmallScreen}
    />
  </section>
{:else}
  <Card>
    <p>Account not provided.</p>
  </Card>
{/if}
