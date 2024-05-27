<script>
  import { navigating, page } from "$app/stores";
  import { BlockDetails, LatestTransactionsCard } from "$lib/components";
  import { duskAPI } from "$lib/services";
  import { appStore } from "$lib/stores";
  import { createPollingDataStore } from "$lib/dusk/svelte-stores";
  import { onNetworkChange } from "$lib/lifecyles";
  import { onDestroy } from "svelte";

  const pollingDataStore = createPollingDataStore(
    duskAPI.getBlock,
    $appStore.fetchInterval
  );

  const updateData = () => {
    pollingDataStore.reset();
    pollingDataStore.start($appStore.network, $page.url.searchParams.get("id"));
  };

  onNetworkChange(updateData);

  $: if (
    $navigating &&
    $navigating.from?.route.id === $navigating.to?.route.id
  ) {
    $navigating.complete.then(updateData);
  }

  $: ({ data, error, isLoading } = $pollingDataStore);
  $: ({ network: currentNetwork } = $appStore);

  onDestroy(pollingDataStore.stop);
</script>

<section class="block">
  <div class="block__details">
    <BlockDetails
      on:retry={() => pollingDataStore.start(currentNetwork)}
      {data}
      {error}
      loading={isLoading}
    />
  </div>
  <div class="block__transactions">
    <LatestTransactionsCard
      on:retry={() => pollingDataStore.start(currentNetwork)}
      txns={data?.transactions.data}
      {error}
      loading={isLoading}
      isOnHomeScreen={false}
    />
  </div>
</section>

<style lang="postcss">
  .block {
    display: flex;
    flex-direction: column;
    row-gap: 1.875em;
  }

  @media (max-width: 768px) {
    .block {
      row-gap: 1.25em;
    }
  }
</style>
