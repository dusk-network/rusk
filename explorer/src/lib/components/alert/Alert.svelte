<script>
  import { mdiAlertOutline, mdiReload } from "@mdi/js";
  import { Button, Icon } from "$lib/dusk/components";
  import { createEventDispatcher } from "svelte";
  import "./Alert.css";

  /** @type {Error | null}*/
  export let error;

  /** @type {Boolean}*/
  export let hasEmptyData;

  const dispatch = createEventDispatcher();

  $: error = error || null;
</script>

<div class="alert">
  <Icon path={mdiAlertOutline} size="large" />
  <header>
    {#if error}
      <h3>There was an error fetching the data.</h3>
      <p class="alert__error-message">{error?.message ?? ""}</p>
    {:else if hasEmptyData}
      <h3>No data to display</h3>
    {/if}
  </header>
  {#if error}
    <Button
      on:click={() => {
        dispatch("retry");
      }}
      icon={{ path: mdiReload, size: "large" }}
      variant="secondary"
    />
  {/if}
</div>
