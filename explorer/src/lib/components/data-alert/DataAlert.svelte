<svelte:options immutable={true} />

<script>
  import { mdiAlertOutline } from "@mdi/js";
  import { Button, Icon } from "$lib/dusk/components";
  import { createEventDispatcher } from "svelte";
  import "./DataAlert.css";

  /** @type {Error | null}*/
  export let error;

  const dispatch = createEventDispatcher();
</script>

<div class="alert">
  <Icon path={mdiAlertOutline} size="large" />
  <header>
    {#if error}
      <p>There was an error fetching the data.</p>
      <p class="alert__error-message">{error.message ?? ""}</p>
    {:else}
      <p>No data to display</p>
    {/if}
  </header>
  {#if error}
    <Button
      on:click={() => {
        dispatch("retry");
      }}
      variant="secondary"
      text="Retry"
    />
  {/if}
</div>
