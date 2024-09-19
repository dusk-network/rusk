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

{#if !!import.meta.env.VITE_CONTRACT_ALLOCATE_DISABLED}
  <IconHeadingCard heading="Shield / Unshield" icons={[mdiSync]}>
    <AllocateContract
      on:operationChange={({ detail }) => updateOperation(detail)}
    />
  </IconHeadingCard>
{/if}
