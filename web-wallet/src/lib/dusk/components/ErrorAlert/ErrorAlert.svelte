<svelte:options immutable={true} />

<script>
  import { mdiCloseThick } from "@mdi/js";

  import { makeClassName } from "$lib/dusk/string";

  import { ErrorDetails, Icon } from "..";

  /** @type {string | undefined} */
  export let className = undefined;

  /** @type {Error} */
  export let error;

  /** @type {GapSize} */
  export let gap = "normal";

  /** @type {string} */
  export let summary;

  $: classes = makeClassName([
    "dusk-error-alert",
    gap !== "normal" ? `dusk-error-alert--${gap}-gap` : "",
    className,
  ]);
</script>

<div {...$$restProps} class={classes}>
  <Icon className="dusk-error-alert__icon" path={mdiCloseThick} size="large" />
  <ErrorDetails className="dusk-error-alert__error-details" {error} {summary} />
</div>
