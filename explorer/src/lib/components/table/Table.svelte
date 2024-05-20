<script>
  import { makeClassName } from "$lib/dusk/string";
  import "./Table.css";

  /** @type {string | Undefined} */
  export let className = undefined;

  $: classes = makeClassName(["table-container", className]);
</script>

<div class={classes}>
  <table class="table">
    <slot />
  </table>
</div>
