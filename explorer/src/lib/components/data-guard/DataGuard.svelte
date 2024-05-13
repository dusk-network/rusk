<script>
  /** @type {*}*/
  export let data;
</script>

{#if data || data === 0}
  <slot />
{:else}
  <span>- - -</span>
{/if}
