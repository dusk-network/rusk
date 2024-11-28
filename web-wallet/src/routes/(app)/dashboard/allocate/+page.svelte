<svelte:options immutable={true} />

<script>
  import { onDestroy } from "svelte";
  import { mdiSync } from "@mdi/js";
  import { AllocateContract } from "$lib/containers";
  import { updateOperation } from "$lib/contracts";
  import { IconHeadingCard } from "$lib/containers/Cards";

  onDestroy(() => {
    updateOperation("");
  });
</script>

{#if import.meta.env.VITE_FEATURE_ALLOCATE || false}
  <IconHeadingCard heading="Allocate" icons={[mdiSync]}>
    <AllocateContract
      on:operationChange={({ detail }) => updateOperation(detail)}
    />
  </IconHeadingCard>
{/if}
