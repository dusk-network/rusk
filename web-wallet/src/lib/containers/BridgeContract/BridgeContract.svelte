<svelte:options immutable={true} />

<script>
  import { mdiHistory } from "@mdi/js";

  import { Icon } from "$lib/dusk/components";
</script>

<article class="bridge">
  <header class="bridge__header">
    <h3 class="h4">Bridge</h3>
    <div class="bridge__header-icons">
      <Icon path={mdiHistory} />
    </div>
  </header>
</article>

<style lang="postcss">
  .bridge {
    border-radius: 1.25em;
    background: var(--surface-color);
    display: flex;
    flex-direction: column;
    gap: var(--default-gap);
    padding: 1.25em;

    &__header {
      display: flex;
      justify-content: space-between;
    }

    &__header-icons {
      display: flex;
      align-items: center;
      gap: 0.675em;
    }
  }
</style>
