<script>
  /* eslint-disable svelte/infinite-reactive-loop */

  /** @type {Number} */
  export let interval = 1000;

  let updateFlag = 0;
  $: setTimeout(() => {
    updateFlag ^= 1;
  }, interval);
</script>

{#key updateFlag}
  <slot />
{/key}
