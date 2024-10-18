<script>
  import { ProvisionersCard } from "$lib/components";
  import { duskAPI } from "$lib/services";
  import { appStore } from "$lib/stores";
  import { createPollingDataStore } from "$lib/dusk/svelte-stores";
  import { onDestroy, onMount } from "svelte";

  const pollingDataStore = createPollingDataStore(
    duskAPI.getProvisioners,
    $appStore.fetchInterval
  );

  onMount(() => pollingDataStore.start());
  onDestroy(pollingDataStore.stop);

  $: ({ data, error, isLoading } = $pollingDataStore);
  $: ({ isSmallScreen } = $appStore);
</script>

<section>
  <ProvisionersCard
    on:retry={() => pollingDataStore.start()}
    provisioners={data}
    {error}
    loading={isLoading}
    {isSmallScreen}
  />
</section>
