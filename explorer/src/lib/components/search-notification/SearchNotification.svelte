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
      >{data.res instanceof Error ? "Error" : "No results found"}</span
    >
    <Button
      className="search-notification__header-action"
      on:click={() => dispatch("close")}
      icon={{
        path: mdiClose,
        position: "after",
        size: "normal",
      }}
    />
  </header>
  <div class="search-notification__content">
    {#if data.res instanceof Error}
      <code>
        {data.res.message}
      </code>
    {/if}
    <span class="search-notification__content-text">
      The search string you entered is: <span
        class="search-notification__content-query">{data.query}</span
      >
    </span>
  </div>
  <div slot="footer">
    <span class="search-notification__footer-text">
      If you think there is a problem with this result, please contact us.
    </span>
  </div>
</Card>
