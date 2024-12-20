<script>
  import { onDestroy, onMount } from "svelte";

  import { Tooltip } from "$lib/dusk/components";
  import { ProvisionersCard } from "$lib/components";
  import { duskAPI } from "$lib/services";
  import { appStore } from "$lib/stores";
  import { createPollingDataStore } from "$lib/dusk/svelte-stores";

  const pollingDataStore = createPollingDataStore(
    duskAPI.getProvisioners,
    $appStore.provisionersFetchInterval
  );

  onMount(() => pollingDataStore.start());
  onDestroy(pollingDataStore.stop);

  $: ({ data, error, isLoading } = $pollingDataStore);
  $: ({ hasTouchSupport, isSmallScreen } = $appStore);
</script>

<section>
  <ProvisionersCard
    on:retry={() => pollingDataStore.start()}
    provisioners={data}
    {error}
    loading={isLoading}
    {isSmallScreen}
  />
  <Tooltip
    defaultDelayShow={hasTouchSupport ? 0 : undefined}
    id="provisioners-tooltip"
  />
</section>

<style lang="postcss">
  :global {
    #provisioners-tooltip {
      word-break: break-all;
    }
  }
</style>
