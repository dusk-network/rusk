<script>
  import { TransactionsCard } from "$lib/components";
  import { duskAPI } from "$lib/services";
  import { appStore } from "$lib/stores";
  import { createPollingDataStore } from "$lib/dusk/svelte-stores";
  import { onDestroy, onMount } from "svelte";

  const pollingDataStore = createPollingDataStore(
    duskAPI.getTransactions,
    $appStore.fetchInterval
  );

  onMount(() => pollingDataStore.start($appStore.transactionsListEntries));
  onDestroy(pollingDataStore.stop);

  $: ({ data, error, isLoading } = $pollingDataStore);
  $: ({ isSmallScreen, transactionsListEntries } = $appStore);
</script>

<section id="transactions">
  <TransactionsCard
    on:retry={() => pollingDataStore.start(transactionsListEntries)}
    txns={data}
    {error}
    loading={isLoading}
    {isSmallScreen}
  />
</section>
