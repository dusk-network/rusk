<svelte:options immutable={true} />

<script>
  import { createEventDispatcher } from "svelte";
  import {
    mdiArrowDownBoldBoxOutline,
    mdiArrowUpBoldBoxOutline,
    mdiDatabaseArrowDownOutline,
    mdiDatabaseOutline,
    mdiHelpCircleOutline,
  } from "@mdi/js";

  import { Button } from "$lib/dusk/components";

  /** @type {ContractOperation[]} */
  export let items;

  const dispatch = createEventDispatcher();

  /** @type {Record<string, string>} */
  const operationIcons = {
    receive: mdiArrowDownBoldBoxOutline,
    send: mdiArrowUpBoldBoxOutline,
    stake: mdiDatabaseOutline,
    unstake: mdiDatabaseArrowDownOutline,
    "withdraw-rewards": mdiDatabaseArrowDownOutline,
  };
</script>

<menu class="contract-operations">
  {#each items as operation (operation.id)}
    <li class="contract-operations__operation">
      <Button
        className="contract-operations__operation-button"
        disabled={operation.disabled}
        icon={{
          path: operationIcons[operation.id] ?? mdiHelpCircleOutline,
          size: "normal",
        }}
        on:click={() => {
          dispatch("operationChange", operation.id);
        }}
        text={operation.label}
        variant={operation.primary ? "primary" : "tertiary"}
      />
    </li>
  {/each}
</menu>

<style lang="postcss">
  .contract-operations {
    list-style-type: none;

    &,
    &__operation {
      width: 100%;
    }

    &__operation {
      & + & {
        margin-top: var(--default-gap);
      }

      & > :global(.contract-operations__operation-button) {
        width: 100%;
        text-align: left;
      }
    }
  }
</style>
