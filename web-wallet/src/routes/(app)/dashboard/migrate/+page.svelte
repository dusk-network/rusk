<svelte:options immutable={true} />

<script>
  import { onDestroy } from "svelte";
  import { MigrateContract } from "$lib/containers";
  import { operationsStore } from "$lib/stores";

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

<MigrateContract />
