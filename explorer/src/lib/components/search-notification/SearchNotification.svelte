<script>
  import { Button, Card } from "$lib/dusk/components";
  import { mdiClose } from "@mdi/js";
  import { createEventDispatcher } from "svelte";
  import { makeClassName } from "$lib/dusk/string";
  import "./SearchNotification.css";

  /** @type {{query: string, res: Array<[]> | Error}} */
  export let data;

  /** @type {string | Undefined} */
  export let className = undefined;

  const dispatch = createEventDispatcher();

  $: classes = makeClassName(["search-notification", className]);
</script>

<Card className={classes}>
  <header slot="header" class="search-notification__header">
    <span class="search-notification__header-text"
      >The search string you entered is: <span
        class="search-notification__content-query">{data.query}</span
      ></span
    >
    <Button
      className="search-notification__header-action"
      on:click={() => dispatch("close")}
      icon={{
        path: mdiClose,
        position: "after",
        size: "normal",
      }}
      variant="tertiary"
    />
  </header>
  {#if data.res instanceof Error}
    <span class="search-notification__content">
      {data.res.message}
    </span>
  {/if}
</Card>
