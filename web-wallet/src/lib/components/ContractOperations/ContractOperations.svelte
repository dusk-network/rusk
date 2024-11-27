<svelte:options immutable={true} />

<script>
  import { createEventDispatcher } from "svelte";
  import {
    mdiArrowDownBoldBoxOutline,
    mdiArrowLeft,
    mdiArrowUpBoldBoxOutline,
    mdiDatabaseArrowDownOutline,
    mdiDatabaseOutline,
    mdiGiftOpenOutline,
    mdiHelpCircleOutline,
  } from "@mdi/js";

  import { AppAnchorButton } from "$lib/components";
  import { Button } from "$lib/dusk/components";

  /** @type {ContractOperation[]} */
  export let items;

  const dispatch = createEventDispatcher();

  /** @type {Record<string, string>} */
  const operationIcons = {
    "claim-rewards": mdiGiftOpenOutline,
    receive: mdiArrowDownBoldBoxOutline,
    send: mdiArrowUpBoldBoxOutline,
    stake: mdiDatabaseOutline,
    unstake: mdiDatabaseArrowDownOutline,
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
        }}
        on:click={() => {
          dispatch("operationChange", operation.id);
        }}
        text={operation.label}
        variant={operation.primary ? "primary" : "tertiary"}
      />
    </li>
  {/each}
  <li class="contract-operations__operation">
    <AppAnchorButton
      className="contract-operations__operation-button"
      href="/dashboard"
      icon={{ path: mdiArrowLeft }}
      on:click={() => {
        dispatch("operationChange", "");
      }}
      text="Back"
      variant="tertiary"
    />
  </li>
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
        margin-top: 1.25rem;
      }

      & > :global(.contract-operations__operation-button) {
        width: 100%;
        text-align: left;
      }
    }
  }
</style>
