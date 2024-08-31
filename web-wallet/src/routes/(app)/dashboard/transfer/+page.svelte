<svelte:options immutable={true} />

<script>
  import { onDestroy } from "svelte";
  import { TransferContract } from "$lib/containers";
  import { IconHeadingCard } from "$lib/containers/Cards";
  import { contractDescriptors, updateOperation } from "$lib/contracts";
  import { mdiSwapVertical } from "@mdi/js";

  onDestroy(() => {
    updateOperation("");
  });
</script>

{#if !!import.meta.env.VITE_CONTRACT_TRANSFER_DISABLED}
  <IconHeadingCard
    gap="medium"
    heading="Transfer"
    iconPath={mdiSwapVertical}
    reverse
  >
    <TransferContract
      descriptor={contractDescriptors[0]}
      on:operationChange={({ detail }) => updateOperation(detail)}
    />
  </IconHeadingCard>
{/if}
