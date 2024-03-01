<script>
  import { Switch } from "$lib/dusk/components";

  /** @param {CustomEvent} event */
  function handleSwitchChange(event) {
    // eslint-disable-next-line no-console
    console.log(`switch changed to ${event.detail}`);
  }

  let value = false;
</script>

<section>
  <div>
    <span>Normal switch ({value}):</span>
    <Switch bind:value on:change={handleSwitchChange} />
  </div>
  <div>
    <span>Readonly switch:</span>
    <Switch readOnly={true} />
  </div>
  <div>
    <span>Disabled switch:</span>
    <Switch disabled={true} />
  </div>
</section>

<style lang="postcss">
  section {
    padding: 0.5em;

    & > div {
      flex: 1;
      display: flex;
      flex-direction: column;
      gap: 0.5em;
    }
  }
</style>
