<svelte:options immutable={true} />

<script>
  import { onDestroy } from "svelte";
  import { TransferContract } from "$lib/containers";
  import { IconHeadingCard } from "$lib/containers/Cards";
  import { contractDescriptors } from "$lib/contracts";
  import { operationsStore } from "$lib/stores";
  import { mdiSwapVertical } from "@mdi/js";

  /** @param {string} id */
  function updateOperation(id) {
    operationsStore.update((store) => ({
      ...store,
      currentOperation: id,
    }));
  }

  onDestroy(() => {
    updateOperation("");
  });
</script>

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
