<script>
  import { makeClassName } from "$lib/dusk/string";
  import "./Badge.css";

  export let text = "";

  /** @type {BadgeVariant} */
  export let variant = "neutral";

  /** @type {String | Undefined} */
  export let className = undefined;

  /** @type {number | Undefined} */
  export let maxlength = undefined;

  $: classes = makeClassName([
    "dusk-badge",
    `dusk-badge--variant-${variant}`,
    className,
  ]);
</script>

<span {...$$restProps} class={classes}>{text.substring(0, maxlength)}</span>
