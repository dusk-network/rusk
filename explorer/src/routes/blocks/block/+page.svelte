<script>
  import { onMount } from "svelte";
  import { navigating, page } from "$app/stores";
  import { BlockDetails, LatestTransactionsCard } from "$lib/components";
  import { duskAPI } from "$lib/services";
  import { appStore } from "$lib/stores";
  import { createDataStore } from "$lib/dusk/svelte-stores";

  const dataStore = createDataStore(duskAPI.getBlock);
  const payloadStore = createDataStore(duskAPI.getBlockDetails);

  const getBlock = () => {
    dataStore.getData($page.url.searchParams.get("id"));
    payloadStore.getData($page.url.searchParams.get("id"));
  };

  onMount(getBlock);

  $: if (
    $navigating &&
    $navigating.from?.route.id === $navigating.to?.route.id
  ) {
    $navigating.complete.then(getBlock);
  }

  $: ({ isSmallScreen } = $appStore);
  $: ({ data, error, isLoading } = $dataStore);
  $: ({ data: payloadData } = $payloadStore);
</script>

<section class="block">
  <div class="block__details">
    <BlockDetails
      on:retry={getBlock}
      {data}
      {error}
      loading={isLoading}
      payload={payloadData}
    />
  </div>
  <div class="block__transactions">
    <LatestTransactionsCard
      on:retry={getBlock}
      txns={data?.transactions.data}
      {error}
      loading={isLoading}
      isOnHomeScreen={false}
      {isSmallScreen}
      displayTooltips={true}
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
